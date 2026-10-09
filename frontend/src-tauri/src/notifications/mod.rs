// Notification system module
pub mod types;
pub mod system;
pub mod settings;
pub mod commands;
pub mod manager;

// Re-export main types for easy access
pub use types::{
    Notification, NotificationType, NotificationPriority, NotificationTimeout
};
pub use settings::{ConsentManager, NotificationSettings};
pub use manager::NotificationManager;
pub use system::SystemNotificationHandler;

// Export commands for Tauri
pub use commands::{
    request_notification_permission,
    show_notification,
    show_test_notification,
    is_dnd_active,
    get_system_dnd_status,
};