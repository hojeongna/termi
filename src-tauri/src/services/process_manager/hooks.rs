use crate::models::terminal::{Activity, Status};

use super::Manager;
use super::types::*;

impl Manager {
    /// Hook 이벤트를 처리하여 터미널 상태를 전환한다.
    /// `session_id`로 특정 터미널을 1:1 매핑하고, `event_name`에 따라 Active↔Idle 전환.
    /// - 이미 session_id가 매핑된 터미널이 있으면 → 그 터미널만 전환
    /// - 새 session_id면 → project_path가 같고 session_id 미할당인 터미널 중 1개에 매핑
    /// 반환: 전환이 발생한 터미널들의 (id, project_path, project_name, terminal_name, new_activity, notification_enabled, monitored) 튜플 목록.
    #[allow(clippy::type_complexity)]
    pub(crate) fn handle_hook_event(
        &self,
        session_id: &str,
        project_path: &str,
        event_name: &str,
    ) -> Vec<TransitionTuple> {
        let mut transitions = Vec::new();

        let mut guard = match self.lock_terminals() {
            Ok(g) => g,
            Err(_) => return transitions,
        };

        // 1단계: session_id로 이미 매핑된 터미널 찾기
        let matched_id = guard.iter()
            .find(|(_, mt)| mt.hook_session_id.as_deref() == Some(session_id))
            .map(|(id, _)| id.clone());

        // 2단계: 매핑된 터미널이 없으면 project_path로 후보 찾기
        let target_id = matched_id.or_else(|| {
            let normalized_path = project_path.to_lowercase().replace('/', "\\");
            let normalized_path = normalized_path.trim_end_matches('\\');

            let is_path_match = |mt: &ManagedTerminal| -> bool {
                mt.instance.status == Status::Running
                    && mt.hook_session_id.is_none()
                    && {
                        let mt_path = mt.instance.project_path.to_lowercase().replace('/', "\\");
                        let mt_path = mt_path.trim_end_matches('\\');
                        mt_path == normalized_path
                    }
            };

            // 우선순위 1: tab_renamed == true (사용자가 탭 이름을 변경 → ✳ 감지 불가, hook 필요)
            // 우선순위 2: title_tracking == false (✳ 미감지, 아직 구분 안 됨)
            // 우선순위 3: 아무 후보
            guard.iter()
                .find(|(_, mt)| is_path_match(mt) && mt.tab_renamed)
                .or_else(|| guard.iter().find(|(_, mt)| is_path_match(mt) && !mt.title_tracking))
                .or_else(|| guard.iter().find(|(_, mt)| is_path_match(mt)))
                .map(|(id, _)| id.clone())
        });

        let target_id = match target_id {
            Some(id) => id,
            None => {
                self.debug_log.push(
                    "Hook",
                    format!("No matching terminal for session={} cwd={}", session_id, project_path),
                );
                return transitions;
            }
        };

        let mt = match guard.get_mut(&target_id) {
            Some(mt) => mt,
            None => return transitions,
        };

        // session_id 매핑 (첫 매칭 시)
        if mt.hook_session_id.is_none() {
            self.debug_log.push(
                "Hook",
                format!(
                    "{}: session_id={} mapped",
                    mt.instance.terminal_name, session_id,
                ),
            );
            mt.hook_session_id = Some(session_id.to_string());
        }

        // monitored를 true로 설정
        mt.instance.monitored = true;

        match event_name {
            "Stop" => {
                if mt.instance.activity != Activity::Idle {
                    self.debug_log.push(
                        "Hook",
                        format!(
                            "{}: {:?} → Idle (hook Stop, session={})",
                            mt.instance.terminal_name, mt.instance.activity, session_id,
                        ),
                    );
                    mt.instance.activity = Activity::Idle;
                    mt.instance.last_idle_at = Some(now_timestamp());
                    mt.pending_active_since = None;
                    transitions.push((
                        target_id,
                        mt.instance.project_path.clone(),
                        mt.instance.project_name.clone(),
                        mt.instance.terminal_name.clone(),
                        Activity::Idle,
                        mt.instance.notification_enabled,
                        mt.instance.monitored,
                        mt.instance.last_idle_at.clone(),
                    ));
                }
            }
            "UserPromptSubmit" => {
                if mt.instance.activity != Activity::Active {
                    self.debug_log.push(
                        "Hook",
                        format!(
                            "{}: {:?} → Active (hook UserPromptSubmit, session={})",
                            mt.instance.terminal_name, mt.instance.activity, session_id,
                        ),
                    );
                    mt.instance.activity = Activity::Active;
                    mt.pending_active_since = None;
                    transitions.push((
                        target_id,
                        mt.instance.project_path.clone(),
                        mt.instance.project_name.clone(),
                        mt.instance.terminal_name.clone(),
                        Activity::Active,
                        mt.instance.notification_enabled,
                        mt.instance.monitored,
                        mt.instance.last_idle_at.clone(),
                    ));
                }
            }
            other => {
                self.debug_log.push(
                    "Hook",
                    format!("Unknown hook event name: {}", other),
                );
            }
        }

        transitions
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::models::terminal::{Activity, Instance, Status};
    use crate::services::debug_log::DebugLog;
    use super::Manager;

    fn make_instance(id: &str, project_id: &str, name: &str) -> Instance {
        Instance {
            id: id.to_string(),
            project_id: project_id.to_string(),
            project_name: name.to_string(),
            project_path: format!("C:\\projects\\{}", name),
            terminal_name: format!("터미널 1"),
            status: Status::Running,
            launched_at: "12345Z".to_string(),
            activity: Activity::Active,
            notification_enabled: true,
            monitored: false,
            attached: false,
            last_idle_at: None,
        }
    }

    fn test_pm() -> Manager {
        Manager::new(Arc::new(DebugLog::new_for_test()))
    }

    #[test]
    fn handle_hook_event_stop_transitions_active_to_idle() {
        let pm = test_pm();
        pm.register_terminal_for_test(make_instance("t1", "p1", "test-project")).unwrap();

        let transitions = pm.handle_hook_event("sess-1", "C:\\projects\\test-project", "Stop");
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].0, "t1");
        assert_eq!(transitions[0].4, Activity::Idle);

        let all = pm.get_all().unwrap();
        assert_eq!(all[0].activity, Activity::Idle);
        assert!(all[0].monitored);
    }

    #[test]
    fn handle_hook_event_stop_ignores_already_idle() {
        let pm = test_pm();
        let mut instance = make_instance("t1", "p1", "test-project");
        instance.activity = Activity::Idle;
        pm.register_terminal_for_test(instance).unwrap();

        let transitions = pm.handle_hook_event("sess-1", "C:\\projects\\test-project", "Stop");
        assert!(transitions.is_empty(), "already Idle → no transition");
    }

    #[test]
    fn handle_hook_event_prompt_transitions_idle_to_active() {
        let pm = test_pm();
        let mut instance = make_instance("t1", "p1", "test-project");
        instance.activity = Activity::Idle;
        pm.register_terminal_for_test(instance).unwrap();

        let transitions = pm.handle_hook_event("sess-1", "C:\\projects\\test-project", "UserPromptSubmit");
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].4, Activity::Active);
    }

    #[test]
    fn handle_hook_event_prompt_ignores_already_active() {
        let pm = test_pm();
        pm.register_terminal_for_test(make_instance("t1", "p1", "test-project")).unwrap();

        let transitions = pm.handle_hook_event("sess-1", "C:\\projects\\test-project", "UserPromptSubmit");
        assert!(transitions.is_empty(), "already Active → no transition");
    }

    #[test]
    fn handle_hook_event_no_matching_terminal() {
        let pm = test_pm();
        pm.register_terminal_for_test(make_instance("t1", "p1", "other-project")).unwrap();

        let transitions = pm.handle_hook_event("sess-1", "C:\\projects\\not-registered", "Stop");
        assert!(transitions.is_empty());
    }

    #[test]
    fn handle_hook_event_session_id_maps_to_one_terminal_only() {
        let pm = test_pm();
        let mut i1 = make_instance("t1", "p1", "shared");
        i1.project_path = "C:\\projects\\shared".to_string();
        let mut i2 = make_instance("t2", "p1", "shared");
        i2.project_path = "C:\\projects\\shared".to_string();
        pm.register_terminal_for_test(i1).unwrap();
        pm.register_terminal_for_test(i2).unwrap();

        // 첫 Stop → session_id가 t1 또는 t2 중 하나에 매핑, 1개만 전환
        let transitions = pm.handle_hook_event("sess-A", "C:\\projects\\shared", "Stop");
        assert_eq!(transitions.len(), 1, "session_id should map to exactly one terminal");

        let transitioned_id = transitions[0].0.clone();

        // 같은 session_id로 다시 → 이미 Idle이므로 무시
        let transitions2 = pm.handle_hook_event("sess-A", "C:\\projects\\shared", "Stop");
        assert!(transitions2.is_empty());

        // 다른 session_id → 나머지 터미널에 매핑
        let transitions3 = pm.handle_hook_event("sess-B", "C:\\projects\\shared", "Stop");
        assert_eq!(transitions3.len(), 1);
        assert_ne!(transitions3[0].0, transitioned_id, "different session should map to different terminal");
    }

    #[test]
    fn handle_hook_event_session_id_persists_across_calls() {
        let pm = test_pm();
        pm.register_terminal_for_test(make_instance("t1", "p1", "test-project")).unwrap();

        // Stop → t1에 매핑, Idle 전환
        pm.handle_hook_event("sess-1", "C:\\projects\\test-project", "Stop");
        // UserPromptSubmit → 같은 session_id이므로 같은 t1, Active 전환
        let transitions = pm.handle_hook_event("sess-1", "C:\\projects\\test-project", "UserPromptSubmit");
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].0, "t1");
        assert_eq!(transitions[0].4, Activity::Active);
    }

    #[test]
    fn handle_hook_event_path_matching_case_insensitive() {
        let pm = test_pm();
        let mut instance = make_instance("t1", "p1", "test-project");
        instance.project_path = "C:\\Projects\\Test-Project".to_string();
        pm.register_terminal_for_test(instance).unwrap();

        let transitions = pm.handle_hook_event("sess-1", "c:\\projects\\test-project", "Stop");
        assert_eq!(transitions.len(), 1);
    }

    #[test]
    fn handle_hook_event_stop_sets_last_idle_at() {
        let pm = test_pm();
        pm.register_terminal_for_test(make_instance("t1", "p1", "test-project")).unwrap();

        let transitions = pm.handle_hook_event("sess-1", "C:\\projects\\test-project", "Stop");
        assert_eq!(transitions.len(), 1);

        // last_idle_at should be set in the transition tuple (8th element)
        assert!(transitions[0].7.is_some(), "last_idle_at should be Some after Idle transition");

        // Verify instance also has last_idle_at set
        let all = pm.get_all().unwrap();
        assert!(all[0].last_idle_at.is_some(), "instance.last_idle_at should be set");
    }

    #[test]
    fn handle_hook_event_prompt_preserves_last_idle_at() {
        let pm = test_pm();
        let mut instance = make_instance("t1", "p1", "test-project");
        instance.activity = Activity::Idle;
        instance.last_idle_at = Some("1710500000Z".to_string());
        pm.register_terminal_for_test(instance).unwrap();

        let transitions = pm.handle_hook_event("sess-1", "C:\\projects\\test-project", "UserPromptSubmit");
        assert_eq!(transitions.len(), 1);

        // last_idle_at should still be the original value (not changed on Active transition)
        assert_eq!(transitions[0].7, Some("1710500000Z".to_string()));

        let all = pm.get_all().unwrap();
        assert_eq!(all[0].last_idle_at, Some("1710500000Z".to_string()));
    }
}
