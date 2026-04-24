pub const COMMON_CLOSE: &str = "common.close";
pub const COMMON_LOADING: &str = "common.loading";

pub const TOP_BAR_SITE_TITLE: &str = "top_bar.site_title";
pub const TOP_BAR_NAV_HOME: &str = "top_bar.nav.home";
pub const TOP_BAR_NAV_ABOUT: &str = "top_bar.nav.about";
pub const TOP_BAR_NAV_ABOUT_BLOG: &str = "top_bar.nav.about_blog";
pub const TOP_BAR_NAV_BLOG: &str = "top_bar.nav.blog";
pub const TOP_BAR_NAV_PHOTOGRAPHS: &str = "top_bar.nav.photographs";
pub const TOP_BAR_NAV_PROJECTS: &str = "top_bar.nav.projects";
pub const TOP_BAR_NAV_VISITOR_BOARD: &str = "top_bar.nav.visitor_board";
pub const TOP_BAR_NAV_GEO_IP: &str = "top_bar.nav.geo_ip";
pub const TOP_BAR_NAV_BACKEND_STATS: &str = "top_bar.nav.backend_stats";
pub const TOP_BAR_AUTH_LOGIN: &str = "top_bar.auth.login";
pub const TOP_BAR_AUTH_LOGOUT: &str = "top_bar.auth.logout";
pub const TOP_BAR_PROFILE_EDIT: &str = "top_bar.profile.edit";
pub const TOP_BAR_MENU_TITLE: &str = "top_bar.menu.title";
pub const TOP_BAR_ARIA_OPEN_SIDEBAR: &str = "top_bar.aria.open_sidebar";
pub const TOP_BAR_ARIA_TOGGLE_THEME: &str = "top_bar.aria.toggle_theme";
pub const TOP_BAR_ARIA_OPEN_USER_MENU: &str = "top_bar.aria.open_user_menu";
pub const TOP_BAR_LANGUAGE_LABEL: &str = "top_bar.language.label";
pub const TOP_BAR_LANGUAGE_ENGLISH: &str = "top_bar.language.english";
pub const TOP_BAR_LANGUAGE_KOREAN: &str = "top_bar.language.korean";

pub const BOTTOM_BAR_SITE_STATUS: &str = "bottom_bar.site_status";
pub const BOTTOM_BAR_FE: &str = "bottom_bar.fe";
pub const BOTTOM_BAR_BE: &str = "bottom_bar.be";
pub const BOTTOM_BAR_BUILT: &str = "bottom_bar.built";
pub const BOTTOM_BAR_WITH_SOLID: &str = "bottom_bar.with_solid";
pub const BOTTOM_BAR_METRICS: &str = "bottom_bar.metrics";
pub const BOTTOM_BAR_UP: &str = "bottom_bar.up";
pub const BOTTOM_BAR_HANDLED: &str = "bottom_bar.handled";
pub const BOTTOM_BAR_RESPONSES: &str = "bottom_bar.responses";
pub const BOTTOM_BAR_SESSIONS: &str = "bottom_bar.sessions";
pub const BOTTOM_BAR_DB: &str = "bottom_bar.db";
pub const BOTTOM_BAR_DB_LATENCY: &str = "bottom_bar.db_latency";
pub const BOTTOM_BAR_STATE_AGE: &str = "bottom_bar.state_age";
pub const BOTTOM_BAR_NET: &str = "bottom_bar.net";
pub const BOTTOM_BAR_TAP: &str = "bottom_bar.tap";
pub const BOTTOM_BAR_OPEN_DETAILS: &str = "bottom_bar.open_details";
pub const BOTTOM_BAR_TIME_TO_REPORT: &str = "bottom_bar.time_to_report";

pub const APP_ERROR_TITLE: &str = "app.error.title";
pub const APP_ERROR_UNKNOWN: &str = "app.error.unknown";
pub const APP_ERROR_TRY_AGAIN: &str = "app.error.try_again";

pub const PAGE_HOME_TITLE: &str = "page.home.title";
pub const PAGE_ABOUT_TITLE: &str = "page.about.title";
pub const PAGE_ABOUT_BLOG_TITLE: &str = "page.about_blog.title";
pub const PAGE_BLOG_LIST_TITLE: &str = "page.blog.list_title";
pub const PAGE_BLOG_NEW_TITLE: &str = "page.blog.new_title";
pub const PAGE_BLOG_EDIT_TITLE: &str = "page.blog.edit_title";
pub const PAGE_LIVE_CHAT_TITLE: &str = "page.live_chat.title";
pub const PAGE_PHOTOGRAPHS_TITLE: &str = "page.photographs.title";
pub const PAGE_PROJECTS_TITLE: &str = "page.projects.title";
pub const PAGE_GEO_IP_TITLE: &str = "page.geo_ip.title";
pub const PAGE_BACKEND_STATS_TITLE: &str = "page.backend_stats.title";
pub const PAGE_LOGIN_TITLE: &str = "page.login.title";
pub const PAGE_SIGNUP_TITLE: &str = "page.signup.title";
pub const PAGE_FIND_PASSWORD_TITLE: &str = "page.find_password.title";
pub const PAGE_RESET_PASSWORD_TITLE: &str = "page.reset_password.title";
pub const PAGE_EDIT_PROFILE_TITLE: &str = "page.edit_profile.title";
pub const PAGE_NOT_FOUND_TITLE: &str = "page.not_found.title";

pub const LIVE_CHAT_OPEN: &str = "live_chat.open";
pub const LIVE_CHAT_ONLINE: &str = "live_chat.online";
pub const LIVE_CHAT_LOAD_OLDER: &str = "live_chat.load_older";
pub const LIVE_CHAT_LOADING: &str = "live_chat.loading";
pub const LIVE_CHAT_MESSAGE_PLACEHOLDER: &str = "live_chat.message_placeholder";
pub const LIVE_CHAT_SEND: &str = "live_chat.send";
pub const LIVE_CHAT_GUEST_IP: &str = "live_chat.guest_ip";

pub const REQUIRED_UI_TEXT_KEYS: &[&str] = &[
    COMMON_CLOSE,
    COMMON_LOADING,
    TOP_BAR_SITE_TITLE,
    TOP_BAR_NAV_HOME,
    TOP_BAR_NAV_ABOUT,
    TOP_BAR_NAV_ABOUT_BLOG,
    TOP_BAR_NAV_BLOG,
    TOP_BAR_NAV_PHOTOGRAPHS,
    TOP_BAR_NAV_PROJECTS,
    TOP_BAR_NAV_VISITOR_BOARD,
    TOP_BAR_NAV_GEO_IP,
    TOP_BAR_NAV_BACKEND_STATS,
    TOP_BAR_AUTH_LOGIN,
    TOP_BAR_AUTH_LOGOUT,
    TOP_BAR_PROFILE_EDIT,
    TOP_BAR_MENU_TITLE,
    TOP_BAR_ARIA_OPEN_SIDEBAR,
    TOP_BAR_ARIA_TOGGLE_THEME,
    TOP_BAR_ARIA_OPEN_USER_MENU,
    TOP_BAR_LANGUAGE_LABEL,
    TOP_BAR_LANGUAGE_ENGLISH,
    TOP_BAR_LANGUAGE_KOREAN,
    BOTTOM_BAR_SITE_STATUS,
    BOTTOM_BAR_FE,
    BOTTOM_BAR_BE,
    BOTTOM_BAR_BUILT,
    BOTTOM_BAR_WITH_SOLID,
    BOTTOM_BAR_METRICS,
    BOTTOM_BAR_UP,
    BOTTOM_BAR_HANDLED,
    BOTTOM_BAR_RESPONSES,
    BOTTOM_BAR_SESSIONS,
    BOTTOM_BAR_DB,
    BOTTOM_BAR_DB_LATENCY,
    BOTTOM_BAR_STATE_AGE,
    BOTTOM_BAR_NET,
    BOTTOM_BAR_TAP,
    BOTTOM_BAR_OPEN_DETAILS,
    BOTTOM_BAR_TIME_TO_REPORT,
    APP_ERROR_TITLE,
    APP_ERROR_UNKNOWN,
    APP_ERROR_TRY_AGAIN,
    PAGE_HOME_TITLE,
    PAGE_ABOUT_TITLE,
    PAGE_ABOUT_BLOG_TITLE,
    PAGE_BLOG_LIST_TITLE,
    PAGE_BLOG_NEW_TITLE,
    PAGE_BLOG_EDIT_TITLE,
    PAGE_LIVE_CHAT_TITLE,
    PAGE_PHOTOGRAPHS_TITLE,
    PAGE_PROJECTS_TITLE,
    PAGE_GEO_IP_TITLE,
    PAGE_BACKEND_STATS_TITLE,
    PAGE_LOGIN_TITLE,
    PAGE_SIGNUP_TITLE,
    PAGE_FIND_PASSWORD_TITLE,
    PAGE_RESET_PASSWORD_TITLE,
    PAGE_EDIT_PROFILE_TITLE,
    PAGE_NOT_FOUND_TITLE,
    LIVE_CHAT_OPEN,
    LIVE_CHAT_ONLINE,
    LIVE_CHAT_LOAD_OLDER,
    LIVE_CHAT_LOADING,
    LIVE_CHAT_MESSAGE_PLACEHOLDER,
    LIVE_CHAT_SEND,
    LIVE_CHAT_GUEST_IP,
];
