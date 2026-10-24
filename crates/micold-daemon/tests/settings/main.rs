//! The daemon's `settings` integration tests, linked as one test binary (#1175): one module per former test file.
#[path = "../support/attention.rs"]
mod attention;
#[path = "../support/conn.rs"]
mod conn;

mod catalog_adoption;
mod diff_layout_setting;
mod github_issues_setting;
mod jira_base_url_setting;
mod jira_setting;
mod pane_layout_store;
mod plan_usage_setting;
mod pr_status_setting;
mod settings_default_ai_cli;
mod settings_default_prompt_template;
mod settings_desktop_notifications;
mod settings_long_task_threshold;
mod settings_service_write_refuses_failed_read;
