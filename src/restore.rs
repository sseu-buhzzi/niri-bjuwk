use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, process};

use itertools::{Itertools, Position};
use niri_ipc::{Action, Window, Workspace, WorkspaceReferenceArg};
use smol_str::{SmolStr, ToSmolStr};

use crate::error::{BjuwkResult, IoContextExt};
use crate::match_config::{MatchAction, MatchConfig};
use crate::niri_ipc_wrapper::{NiriIpcWrapper, WorkspaceId};
use crate::path_util::PathManager;
use crate::snapshot::Snapshot;

pub fn execute(
    snapshot_path: Option<PathBuf>,
    match_config_path: Option<PathBuf>,
    dry_run: bool,
) -> BjuwkResult<()> {
    let snapshot_path = snapshot_path.unwrap_or_else(|| PathManager::get().snapshot_link_path());
    let snap = Snapshot::load(&snapshot_path)?;
    let Snapshot {
        workspaces: swses,
        windows: swins,
        ..
    } = snap;

    let match_config_path =
        match_config_path.unwrap_or_else(|| PathManager::get().match_config_path());
    let config = load_config_or_fallback(&match_config_path)?;

    let niw = NiriIpcWrapper::new()?;
    let (_, owins) = niw.receive_workspaces_and_windows()?;

    let mut niw = NiriIpcWrapper::new()?;
    let orig_focused = niw.get_focused_window()?;
    scopeguard::defer! {
        if let Err(e) = (|| {
            if let Some(Window { id, .. }) = orig_focused {
                let mut niw = NiriIpcWrapper::new()?;
                niw.send_action(Action::FocusWindow { id })?;
            }
            BjuwkResult::Ok(())
        })() {
            eprintln!("Failed to restore the original focus: {e}");
        }
    }

    let reconstruction = reconstruct_windows(&config, &swins, &swses, &owins)?;
    let stage_ws_name = format!("__niri_bjuwk_stage_{}", process::id());
    let stage_ws_ref = WorkspaceReferenceArg::Name(stage_ws_name.clone());
    for ((mon, ws_idx), cols) in reconstruction {
        niw.send_action(Action::FocusMonitor {
            output: mon.to_string(),
        })?;
        niw.send_action(Action::FocusWorkspace {
            reference: WorkspaceReferenceArg::Index(u8::MAX),
        })?;
        niw.send_action(Action::SetWorkspaceName {
            name: stage_ws_name.clone(),
            workspace: None,
        })?;
        scopeguard::defer! {
            if let Err(e) = (|| {
                let mut niw = NiriIpcWrapper::new()?;
                niw.send_action(Action::MoveWorkspaceToIndex {
                    index: ws_idx as _,
                    reference: Some(stage_ws_ref.clone()),
                })?;
                niw.send_action(Action::UnsetWorkspaceName {
                    reference: Some(stage_ws_ref.clone()),
                })?;
                BjuwkResult::Ok(())
            })() {
                eprintln!("Failed to clean up {stage_ws_name}: {e}");
            }
        }

        for tiles in cols {
            for (pos, &owin) in tiles.iter().with_position() {
                niw.send_action(Action::MoveWindowToWorkspace {
                    window_id: Some(owin.id),
                    reference: stage_ws_ref.clone(),
                    focus: false,
                })?;
                match pos {
                    Position::First | Position::Only => {
                        niw.send_action(Action::FocusColumnLast {})?;
                    }
                    Position::Middle | Position::Last => {
                        niw.send_action(Action::ConsumeWindowIntoColumn {})?;
                    }
                }
            }
        }
    }

    println!("Restore complete.");
    Ok(())
}

fn load_config_or_fallback(path: &Path) -> BjuwkResult<MatchConfig> {
    let config = if path.exists() {
        let config_str = fs::read_to_string(path).context(path.display().to_string())?;
        let config = config_str.parse()?;
        println!("Loaded match config from {}", path.display());
        config
    } else {
        let config = MatchConfig::default();
        println!("Loaded empty match config");
        config
    };
    Ok(config)
}

fn reconstruct_windows<'a>(
    config: &MatchConfig,
    swins: &[Window],
    swses: &[Workspace],
    owins: &'a [Window],
) -> BjuwkResult<HashMap<WorkspacePosition, Vec<Vec<&'a Window>>>> {
    let sws_id_to_pos = map_workspace_id_to_position(swses);

    let remaining_owin_map = owins
        .iter()
        .map(|w| serde_json::to_value(w).map(|v| (w, v)))
        .collect::<serde_json::Result<Vec<_>>>()?;
    let mut remaining_swin_map = swins
        .iter()
        .map(|w| serde_json::to_value(w).map(|v| (w.id, (w, v))))
        .collect::<serde_json::Result<HashMap<_, _>>>()?;

    let mut swin_to_owin = HashMap::new();
    for rule in config.rule_arr() {
        for &(owin, ref owin_val) in &remaining_owin_map {
            if !rule.select(owin_val)? {
                continue;
            }

            let mut swin_to_remove = None;
            for (&swin_id, &(swin, ref swin_val)) in &remaining_swin_map {
                if !(rule.select(swin_val)? && rule.test(swin_val, owin_val)?) {
                    continue;
                }

                swin_to_remove = Some(swin_id);

                match rule.action() {
                    MatchAction::MoveToSaved => {
                        swin_to_owin.insert(swin.id, owin);
                    }
                    MatchAction::Ignore => {}
                }

                break;
            }
            if let Some(swin_id) = swin_to_remove {
                remaining_swin_map.remove(&swin_id);
            }
        }
    }

    let reconstruction = swins
        .iter()
        .filter_map(|swin| {
            let &owin = swin_to_owin.get(&swin.id)?;
            let ws_pos = sws_id_to_pos.get(&swin.workspace_id)?;
            let tile_pos = swin.layout.pos_in_scrolling_layout?;
            Some((ws_pos.clone(), (tile_pos, swin, owin)))
        })
        .into_grouping_map()
        .fold(
            Vec::<Vec<_>>::new(),
            |mut cols, _, ((col, tile), _, owin)| {
                while cols.len() < col {
                    cols.push(vec![]);
                }
                if let Some(tiles) = cols.get_mut(col - 1) {
                    tiles.insert(tiles.len().min(tile - 1), owin);
                }
                cols
            },
        );
    Ok(reconstruction)
}

type WorkspacePosition = (SmolStr, u8);

fn map_workspace_id_to_position<'a>(
    wses: impl IntoIterator<Item = &'a Workspace>,
) -> HashMap<Option<WorkspaceId>, WorkspacePosition> {
    wses.into_iter()
        .filter_map(|ws| {
            ws.output
                .as_ref()
                .map(|o| (Some(ws.id), (o.to_smolstr(), ws.idx)))
        })
        .collect()
}
