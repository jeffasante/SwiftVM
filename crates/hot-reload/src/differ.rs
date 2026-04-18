use vm_core::{Function, Program};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateMigration {
    pub added_with_defaults: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ReloadPlan {
    NoChanges,
    Light {
        changed_functions: Vec<Function>,
        state_migration: StateMigration,
    },
    Hard { reason: String },
}

pub fn build_reload_plan(old_program: &Program, new_program: &Program) -> ReloadPlan {
    if let Some(reason) = state_layout_breaking_change(old_program, new_program) {
        return ReloadPlan::Hard { reason };
    }

    let old_keys: std::collections::HashSet<_> = old_program.functions.keys().collect();
    let new_keys: std::collections::HashSet<_> = new_program.functions.keys().collect();
    let removed: Vec<_> = old_keys.difference(&new_keys).map(|k| (*k).clone()).collect();
    if !removed.is_empty() {
        return ReloadPlan::Hard {
            reason: format!(
                "function removed: {}",
                removed
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
    }

    let mut changed = Vec::new();

    for (name, new_fn) in &new_program.functions {
        let Some(old_fn) = old_program.functions.get(name) else {
            changed.push(new_fn.clone());
            continue;
        };

        if old_fn.params != new_fn.params {
            return ReloadPlan::Hard {
                reason: format!("signature changed for `{name}`"),
            };
        }

        if old_fn.instructions != new_fn.instructions {
            changed.push(new_fn.clone());
        }
    }

    let state_migration = collect_state_migration(old_program, new_program);
    
    // Also check if initial state values changed
    let mut state_value_changed = false;
    for (name, new_field) in &new_program.state_layout {
        if let Some(old_field) = old_program.state_layout.get(name) {
            if old_field.default_value != new_field.default_value {
                state_value_changed = true;
                break;
            }
        }
    }

    if changed.is_empty() && state_migration.added_with_defaults.is_empty() && !state_value_changed {
        ReloadPlan::NoChanges
    } else {
        ReloadPlan::Light {
            changed_functions: changed,
            state_migration,
        }
    }
}

fn state_layout_breaking_change(old_program: &Program, new_program: &Program) -> Option<String> {
    for (name, old_field) in &old_program.state_layout {
        let Some(new_field) = new_program.state_layout.get(name) else {
            return Some(format!("state field `{name}` was removed"));
        };

        if old_field.type_name != new_field.type_name {
            return Some(format!(
                "state field `{name}` changed type from {} to {}",
                old_field.type_name, new_field.type_name
            ));
        }
    }

    for (name, new_field) in &new_program.state_layout {
        if !old_program.state_layout.contains_key(name) && new_field.default_value.is_none() {
            return Some(format!(
                "new state field `{name}` has no default (cannot preserve state)"
            ));
        }
    }

    None
}

fn collect_state_migration(old_program: &Program, new_program: &Program) -> StateMigration {
    let mut added_with_defaults = Vec::new();

    for (name, new_field) in &new_program.state_layout {
        if !old_program.state_layout.contains_key(name) && new_field.default_value.is_some() {
            added_with_defaults.push(name.clone());
        }
    }

    added_with_defaults.sort();
    StateMigration { added_with_defaults }
}

pub fn apply_light_reload(program: &mut Program, new_program: &Program, changed_functions: &[Function]) {
    program.state_layout = new_program.state_layout.clone();
    for function in changed_functions {
        program.replace_or_add_function(function.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vm_core::{Function, Instruction, StateField, Value};

    fn base_program() -> Program {
        let mut p = Program::with_entry("main");
        p.upsert_state_field(StateField {
            name: "counter".into(),
            type_name: "Int".into(),
            default_value: Some(Value::Int(0)),
        });
        p.add_function(Function::new(
            "main",
            vec![],
            vec![Instruction::LoadConst(Value::Int(1)), Instruction::Return],
        ));
        p
    }

    #[test]
    fn identifies_no_changes() {
        let old = base_program();
        let new = base_program();
        assert!(matches!(build_reload_plan(&old, &new), ReloadPlan::NoChanges));
    }

    #[test]
    fn identifies_light_reload() {
        let old = base_program();
        let mut new = base_program();
        new.replace_or_add_function(Function::new(
            "main",
            vec![],
            vec![Instruction::LoadConst(Value::Int(2)), Instruction::Return],
        ));

        match build_reload_plan(&old, &new) {
            ReloadPlan::Light {
                changed_functions,
                state_migration,
            } => {
                assert_eq!(changed_functions.len(), 1);
                assert!(state_migration.added_with_defaults.is_empty());
            }
            _ => panic!("expected light plan"),
        }
    }

    #[test]
    fn identifies_hard_reload_for_signature_change() {
        let old = base_program();
        let mut new = Program::with_entry("main");
        new.upsert_state_field(StateField {
            name: "counter".into(),
            type_name: "Int".into(),
            default_value: Some(Value::Int(0)),
        });
        new.add_function(Function::new(
            "main",
            vec!["value"],
            vec![Instruction::LoadVar("value".into()), Instruction::Return],
        ));

        assert!(matches!(
            build_reload_plan(&old, &new),
            ReloadPlan::Hard { .. }
        ));
    }

    #[test]
    fn identifies_hard_reload_for_state_type_change() {
        let old = base_program();
        let mut new = base_program();
        new.upsert_state_field(StateField {
            name: "counter".into(),
            type_name: "String".into(),
            default_value: Some(Value::Str("0".into())),
        });
        assert!(matches!(
            build_reload_plan(&old, &new),
            ReloadPlan::Hard { .. }
        ));
    }

    #[test]
    fn identifies_light_reload_for_added_function() {
        let old = base_program();
        let mut new = base_program();
        new.add_function(Function::new(
            "helper",
            vec![],
            vec![Instruction::LoadConst(Value::Int(7)), Instruction::Return],
        ));

        match build_reload_plan(&old, &new) {
            ReloadPlan::Light {
                changed_functions,
                state_migration,
            } => {
                assert_eq!(changed_functions.len(), 1);
                assert_eq!(changed_functions[0].name, "helper");
                assert!(state_migration.added_with_defaults.is_empty());
            }
            _ => panic!("expected light plan"),
        }
    }

    #[test]
    fn identifies_hard_reload_for_removed_function() {
        let mut old = base_program();
        old.add_function(Function::new(
            "helper",
            vec![],
            vec![Instruction::LoadConst(Value::Int(1)), Instruction::Return],
        ));
        let new = base_program();

        assert!(matches!(
            build_reload_plan(&old, &new),
            ReloadPlan::Hard { .. }
        ));
    }

    #[test]
    fn identifies_light_reload_for_additive_state_with_default() {
        let old = base_program();
        let mut new = base_program();
        new.upsert_state_field(StateField {
            name: "title".into(),
            type_name: "String".into(),
            default_value: Some(Value::Str("Hello".into())),
        });

        match build_reload_plan(&old, &new) {
            ReloadPlan::Light {
                changed_functions,
                state_migration,
            } => {
                assert!(changed_functions.is_empty());
                assert_eq!(state_migration.added_with_defaults, vec!["title".to_string()]);
            }
            _ => panic!("expected light plan"),
        }
    }
}
