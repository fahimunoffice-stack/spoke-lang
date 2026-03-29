// src/lexer/token.rs
// All possible tokens in the Spoke language

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Literals ──────────────────────────────
    StringLit(String),
    NumberLit(f64),
    Identifier(String),

    // ── Keywords ──────────────────────────────
    // Top-level declarations
    App,
    Page,
    Api,
    Service,
    Component,

    // Data
    Has,
    Data,

    // Auth
    Auth,
    Login,
    Logout,
    Signup,
    Allow,
    Remember,
    Magic,
    Link,

    // Behavior
    Can,
    When,
    Every,
    After,
    Require,
    Behavior,

    // Actors
    Users,
    User,
    Admins,
    Admin,
    Anyone,
    Only,

    // Verbs
    Create,
    Edit,
    Update,
    Delete,
    View,
    Upload,
    Send,
    Show,
    Store,
    Mark,
    Search,
    Record,

    // Field modifiers
    Unique,
    Optional,
    Private,
    Required,
    Indexed,

    // UI
    Pages,
    Navigation,
    Form,
    To,
    As,
    In,
    By,
    From,

    // Logic
    If,
    Unless,
    And,
    Or,
    Not,
    Is,
    Are,
    Their,
    Own,
    Any,

    // HTTP methods
    Get,
    Post,
    Put,
    Patch,

    // Time
    Day,
    Hour,
    Week,
    Month,
    At,
    On,
    Am,
    Pm,

    // Misc
    With,
    Go,
    Also,
    Latest,
    Top,
    Sorted,
    Where,
    Notify,

    // ── Punctuation ───────────────────────────
    Colon,       // :
    Comma,       // ,
    Arrow,       // → or ->
    Apostrophe,  // '
    NewLine,
    Indent,
    Dedent,
    Eof,
}

impl Token {
    /// Try to convert a bare word into a keyword token.
    /// Returns None if it's not a keyword (i.e. it's an Identifier).
    pub fn keyword(word: &str) -> Option<Token> {
        match word.to_lowercase().as_str() {
            "app"       => Some(Token::App),
            "page"      => Some(Token::Page),
            "api"       => Some(Token::Api),
            "service"   => Some(Token::Service),
            "component" => Some(Token::Component),
            "has"       => Some(Token::Has),
            "data"      => Some(Token::Data),
            "auth"      => Some(Token::Auth),
            "login"     => Some(Token::Login),
            "logout"    => Some(Token::Logout),
            "signup"    => Some(Token::Signup),
            "allow"     => Some(Token::Allow),
            "remember"  => Some(Token::Remember),
            "magic"     => Some(Token::Magic),
            "link"      => Some(Token::Link),
            "can"       => Some(Token::Can),
            "when"      => Some(Token::When),
            "every"     => Some(Token::Every),
            "after"     => Some(Token::After),
            "require"   => Some(Token::Require),
            "behavior"  => Some(Token::Behavior),
            "users"     => Some(Token::Users),
            "user"      => Some(Token::User),
            "admins"    => Some(Token::Admins),
            "admin"     => Some(Token::Admin),
            "anyone"    => Some(Token::Anyone),
            "only"      => Some(Token::Only),
            "create"    => Some(Token::Create),
            "edit"      => Some(Token::Edit),
            "update"    => Some(Token::Update),
            "delete"    => Some(Token::Delete),
            "view"      => Some(Token::View),
            "upload"    => Some(Token::Upload),
            "send"      => Some(Token::Send),
            "show"      => Some(Token::Show),
            "store"     => Some(Token::Store),
            "mark"      => Some(Token::Mark),
            "search"    => Some(Token::Search),
            "record"    => Some(Token::Record),
            "unique"    => Some(Token::Unique),
            "optional"  => Some(Token::Optional),
            "private"   => Some(Token::Private),
            "required"  => Some(Token::Required),
            "indexed"   => Some(Token::Indexed),
            "pages"     => Some(Token::Pages),
            "navigation"=> Some(Token::Navigation),
            "form"      => Some(Token::Form),
            "to"        => Some(Token::To),
            "as"        => Some(Token::As),
            "in"        => Some(Token::In),
            "by"        => Some(Token::By),
            "from"      => Some(Token::From),
            "if"        => Some(Token::If),
            "unless"    => Some(Token::Unless),
            "and"       => Some(Token::And),
            "or"        => Some(Token::Or),
            "not"       => Some(Token::Not),
            "is"        => Some(Token::Is),
            "are"       => Some(Token::Are),
            "their"     => Some(Token::Their),
            "own"       => Some(Token::Own),
            "any"       => Some(Token::Any),
            "get"       => Some(Token::Get),
            "post"      => Some(Token::Post),
            "put"       => Some(Token::Put),
            "patch"     => Some(Token::Patch),
            "day"       => Some(Token::Day),
            "hour"      => Some(Token::Hour),
            "week"      => Some(Token::Week),
            "month"     => Some(Token::Month),
            "at"        => Some(Token::At),
            "on"        => Some(Token::On),
            "am"        => Some(Token::Am),
            "pm"        => Some(Token::Pm),
            "with"      => Some(Token::With),
            "go"        => Some(Token::Go),
            "also"      => Some(Token::Also),
            "latest"    => Some(Token::Latest),
            "top"       => Some(Token::Top),
            "sorted"    => Some(Token::Sorted),
            "where"     => Some(Token::Where),
            "notify"    => Some(Token::Notify),
            _           => None,
        }
    }
}

/// A token with its source location for error reporting
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

impl SpannedToken {
    pub fn new(token: Token, line: usize, col: usize) -> Self {
        SpannedToken { token, line, col }
    }
}
