// src/ast/mod.rs
// The Intent Abstract Syntax Tree (IAST).
// Every .spoke program compiles into this tree.

use serde::{Deserialize, Serialize};

// ─── Root ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}

// ─── Top-level Declarations ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Declaration {
    App(AppDecl),
    Page(PageDecl),
    Api(ApiDecl),
    Service(ServiceDecl),
    Component(ComponentDecl),
}

// ─── App ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDecl {
    pub name: String,
    pub auth: Option<AuthBlock>,
    pub data: Vec<EntityDecl>,
    pub behavior: Vec<BehaviorStmt>,
    pub pages: Vec<PageDecl>,
    pub navigation: Option<NavigationBlock>,
    pub services: Vec<ServiceDecl>,
}

// ─── Auth ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthBlock {
    pub methods: Vec<AuthMethod>,
    pub session: Option<SessionConfig>,
    pub redirects: Vec<RedirectConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    EmailPassword {
        signup_fields: Vec<String>,
    },
    MagicLink,
    OAuth {
        provider: OAuthProvider,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OAuthProvider {
    Google,
    GitHub,
    Facebook,
    Apple,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub duration_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectConfig {
    pub event: AuthEvent,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthEvent {
    Login,
    Logout,
    Signup,
}

// ─── Data / Entities ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub relations: Vec<RelationDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDecl {
    pub name: String,
    pub modifiers: Vec<FieldModifier>,
    // Type is NOT declared in .spoke — inferred by rule engine
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldModifier {
    Unique,
    Optional,
    Private,
    Required,
    Indexed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDecl {
    pub from: String,
    pub kind: RelationKind,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationKind {
    BelongsTo,
    HasMany,
    HasOne,
}

// ─── Behavior ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BehaviorStmt {
    Action(ActionStmt),
    Trigger(TriggerStmt),
    Schedule(ScheduleStmt),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStmt {
    pub actor: Actor,
    pub verb: Verb,
    pub object: Object,
    pub condition: Option<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Actor {
    Users,
    Admins,
    Anyone,
    OnlyLoggedIn,
    Role(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Verb {
    Create,
    Edit,
    Update,
    Delete,
    View,
    Upload,
    Send,
    Mark,
    Search,
    Record,
    Login,
    Logout,
    Signup,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Object {
    TheirOwn(String),    // "their own tasks"
    Any(String),         // "any task"
    Entity(String),      // "tasks"
    ByVoice(String),     // "dream by voice"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    ActorOwns(String),
    ActorIs(String),
    FieldIs(String, String),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerStmt {
    pub event: TriggerEvent,
    pub actions: Vec<ActionBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerEvent {
    UserSignsUp,
    UserLogsIn,
    UserLogsOut,
    EntityCreated(String),
    EntityUpdated(String),
    EntityDeleted(String),
    FieldReaches(String, String, String), // entity, field, value
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleStmt {
    pub interval: ScheduleInterval,
    pub actions: Vec<ActionBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleInterval {
    EveryHour,
    EveryDay { time: Option<String> },
    EveryWeek { day: String, time: Option<String> },
    Every { amount: u32, unit: TimeUnit },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
    Months,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionBody {
    Notify { target: String, message: String },
    SendEmail { to: String, subject: Option<String> },
    SendPush { to: String },
    Store { what: String, where_: String },
    Process { what: String, how: String },
    Raw(String),
}

// ─── Pages / UI ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDecl {
    pub name: String,
    pub access: AccessLevel,
    pub body: Vec<PageStmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessLevel {
    Public,
    RequireLogin,
    RequireRole(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageStmt {
    Show(ShowStmt),
    Form(FormDecl),
    Layout(LayoutStmt),
    Action(ActionStmt),
    Redirect(RedirectConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowStmt {
    pub data: DataExpr,
    pub options: Vec<DisplayOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataExpr {
    UserOwned(String),              // "user's tasks"
    All(String),                    // "all tasks"
    Latest(u32, String),            // "latest 5 notifications"
    Top(u32, String, String),       // "top 10 users by score"
    Filtered(String, Condition),    // "tasks where status is pending"
    Entity(String),                 // "tasks"
    Component(String),              // "voice recorder"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisplayOption {
    As(String),                    // "as cards"
    SortedBy(String, SortDir),     // "sorted by deadline"
    In(String),                    // "in a grid"
    Paginate(u32),                 // "20 per page"
    From(TimeRange),               // "from this week"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeRange {
    Today,
    ThisWeek,
    ThisMonth,
    Last(u32, TimeUnit),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormDecl {
    pub purpose: String,           // "to create task"
    pub fields: Vec<FormField>,
    pub on_submit: Option<ActionBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutStmt {
    pub region: LayoutRegion,
    pub body: Vec<PageStmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutRegion {
    LeftSidebar,
    RightPanel,
    MainContent,
    Header,
    Footer,
}

// ─── Navigation ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationBlock {
    pub items: Vec<NavItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NavItem {
    Link { label: String, path: String },
    LogoutButton,
}

// ─── API ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDecl {
    pub version: String,
    pub routes: Vec<ApiRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRoute {
    pub method: HttpMethod,
    pub path: String,
    pub response: String,
    pub access: Option<Actor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

// ─── Service ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDecl {
    pub name: String,
    pub body: Vec<BehaviorStmt>,
}

// ─── Component ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDecl {
    pub name: String,
    pub body: Vec<PageStmt>,
}
