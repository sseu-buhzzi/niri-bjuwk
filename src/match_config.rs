use crate::error::{BjuwkError, BjuwkResult};
use jaq_all::data::Filter;
use jaq_json::Val;
use serde::Deserialize;
use serde_json::{Value, json};
use std::iter;
use std::str::FromStr;

#[derive(Default)]
pub struct MatchConfig {
    rule_arr: Vec<MatchRule>,
}

impl MatchConfig {
    pub fn rule_arr(&self) -> &[MatchRule] {
        &self.rule_arr
    }
}

impl FromStr for MatchConfig {
    type Err = BjuwkError;

    fn from_str(s: &str) -> BjuwkResult<Self> {
        let raw_config = serde_yml::from_str::<RawConfig>(s)?;
        let rule_arr = raw_config
            .rule
            .into_iter()
            .enumerate()
            .map(|(id, raw)| MatchRule::from_raw(id as RuleId, raw))
            .collect::<BjuwkResult<_>>()?;
        Ok(Self { rule_arr })
    }
}

pub struct MatchRule {
    id: RuleId,
    select: Filter,
    test: Filter,
    action: MatchAction,
}

impl MatchRule {
    fn from_raw(id: RuleId, raw: RawRule) -> BjuwkResult<Self> {
        let select = jaq_all::data::compile(&raw.select).map_err(|report_arr| {
            BjuwkError::Other(format!("jaq compile error: {report_arr:?}"))
        })?;
        let test = jaq_all::data::compile(&raw.test).map_err(|report_arr| {
            BjuwkError::Other(format!("jaq compile error: {report_arr:?}"))
        })?;
        let action = MatchAction::from_str(&raw.action)?;
        Ok(Self {
            id,
            select,
            test,
            action,
        })
    }

    pub fn id(&self) -> RuleId {
        self.id
    }

    pub fn select(&self, window: &Value) -> BjuwkResult<bool> {
        Self::eval_bool_jq(&self.select, window)
    }

    pub fn test(&self, saved_window: &Value, opened_window: &Value) -> BjuwkResult<bool> {
        Self::eval_bool_jq(&self.test, &json!([saved_window, opened_window]))
    }

    pub fn action(&self) -> MatchAction {
        self.action
    }

    fn eval_bool_jq(filter: &Filter, value: &Value) -> BjuwkResult<bool> {
        let runner = Default::default();
        let vars = Default::default();
        let input = serde_json::from_value(value.clone())?;
        let mut matched = false;
        jaq_all::data::run(
            &runner,
            filter,
            vars,
            iter::once(Ok::<_, &str>(input)),
            |s: String| BjuwkError::Other(s),
            |v| {
                let v = v.map_err(|e| BjuwkError::Other(format!("{e}")))?;
                matched |= !matches!(v, Val::Null | Val::Bool(false));
                Ok(())
            },
        )?;
        Ok(matched)
    }
}

pub type RuleId = u64;

#[derive(Clone, Copy, Debug)]
pub enum MatchAction {
    MoveToSaved,
    Ignore,
}

impl FromStr for MatchAction {
    type Err = BjuwkError;

    fn from_str(s: &str) -> BjuwkResult<Self> {
        match s {
            "move-to-saved" => Ok(Self::MoveToSaved),
            "ignore" => Ok(Self::Ignore),
            _ => Err(BjuwkError::Other(format!("Unknown MatchAction: {s}"))),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct RawConfig {
    rule: Vec<RawRule>,
}

#[derive(Clone, Debug, Deserialize)]
struct RawRule {
    select: String,
    test: String,
    action: String,
}

#[cfg(test)]
mod tests {
    use crate::match_config::{MatchAction, MatchConfig};
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_match_action_from_str() {
        assert!(matches!(
            MatchAction::from_str("move-to-saved"),
            Ok(MatchAction::MoveToSaved),
        ));

        assert!(matches!(
            MatchAction::from_str("ignore"),
            Ok(MatchAction::Ignore),
        ));

        assert!(
            MatchAction::from_str("unknown one")
                .unwrap_err()
                .to_string()
                .contains("Unknown action"),
        );
    }

    #[test]
    fn test_match_rule() {
        let config = MatchConfig::from_str(
            r#"
                rule:
                  - select: |-
                      .app_id | test("firefox(-esr)?")
                    test: |-
                      map(.title | match("^\\[(.*?)\\] ").captures[0].string)
                      | length == 2 and (unique | length) == 1
                    action: move-to-saved
            "#,
        )
        .expect("Failed to parse RawRule");
        let rule = config
            .rule_arr()
            .first()
            .expect("Failed to get the first rule");

        assert!(matches!(
            rule.test(
                &json!({
                    "app_id": "firefox-esr",
                    "title": "[abc def] foo",
                }),
                &json!({
                    "app_id": "firefox",
                    "title": "[abc def] 123]"
                }),
            ),
            Ok(true)
        ));
    }
}
