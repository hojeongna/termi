use std::collections::HashMap;

use tauri::{AppHandle, Emitter};
use tauri_winrt_notification::Toast;

use crate::error::AppError;
use crate::models::terminal::Activity;

use crate::events;

/// 미확인(pending) 알림 정보
#[derive(Debug, Clone)]
pub(crate) struct PendingNotification;

/// 알림 서비스. 토스트 알림 발생 및 미확인 알림 추적을 담당한다.
pub(crate) struct Notifier {
    pending: HashMap<String, PendingNotification>,
    language: String,
}

impl Notifier {
    /// 지정 언어로 Notifier 인스턴스를 생성한다.
    pub(crate) fn new(language: &str) -> Self {
        Self {
            pending: HashMap::new(),
            language: language.to_string(),
        }
    }

    /// 언어 설정을 변경한다.
    pub(crate) fn set_language(&mut self, language: &str) {
        self.language = language.to_string();
    }

    /// 상태 변경 시 윈도우 토스트 알림을 발생시키고 pending에 추가한다.
    /// Idle 상태일 때만 알림을 발생시킨다.
    pub(crate) fn send_status_notification(
        &mut self,
        app: &AppHandle,
        terminal_id: &str,
        project_name: &str,
        terminal_name: &str,
        status: &Activity,
    ) -> Result<(), AppError> {
        let (title, body) = match notification_message(project_name, terminal_name, status, &self.language) {
            Some(msg) => msg,
            None => return Ok(()),
        };

        // Windows 토스트 알림: tauri-winrt-notification 직접 사용
        // on_activated 콜백으로 알림 클릭 감지 → Tauri 이벤트 emit
        let app_clone = app.clone();
        let tid = terminal_id.to_string();
        Toast::new(Toast::POWERSHELL_APP_ID)
            .title(&title)
            .text1(&body)
            .on_activated(move |_action| {
                let _ = app_clone.emit(events::NOTIFICATION_CLICKED, &tid);
                Ok(())
            })
            .show()
            .map_err(|e| AppError::Notification(format!("알림 발생 실패: {:?}", e)))?;

        self.pending.insert(terminal_id.to_string(), PendingNotification);

        Ok(())
    }

    /// 알림을 확인(acknowledge) 처리하여 pending에서 제거한다.
    pub(crate) fn acknowledge(&mut self, terminal_id: &str) {
        self.pending.remove(terminal_id);
    }
}

/// 상태에 따른 알림 메시지를 생성한다.
/// Idle 상태일 때만 알림 메시지를 반환한다.
pub(crate) fn notification_message(
    project_name: &str,
    terminal_name: &str,
    status: &Activity,
    language: &str,
) -> Option<(String, String)> {
    match status {
        Activity::Idle => {
            let body = match language {
                "ko" => "작업이 끝났습니다!",
                _ => "Task completed!",
            };
            Some((
                format!("{} - {}", project_name, terminal_name),
                body.to_string(),
            ))
        }
        Activity::Active => None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_message_returns_none_for_active_status() {
        let result = notification_message("my-project", "터미널 1", &Activity::Active, "ko");
        assert!(result.is_none());
    }

    #[test]
    fn notification_message_returns_korean_body_for_ko() {
        let result = notification_message("my-project", "터미널 1", &Activity::Idle, "ko");
        assert!(result.is_some());
        let (title, body) = result.unwrap();
        assert_eq!(title, "my-project - 터미널 1");
        assert_eq!(body, "작업이 끝났습니다!");
    }

    #[test]
    fn notification_message_returns_english_body_for_en() {
        let (_, body) = notification_message("my-project", "Terminal 1", &Activity::Idle, "en").unwrap();
        assert_eq!(body, "Task completed!");
    }

    #[test]
    fn notification_message_title_includes_project_name() {
        let (title, _) = notification_message("테스트-프로젝트", "터미널 1", &Activity::Idle, "ko").unwrap();
        assert_eq!(title, "테스트-프로젝트 - 터미널 1");
    }
}
