// src/rules/mod.rs
// Rule Engine — deterministic, zero-LLM resolution of Intent AST nodes.
// Takes raw AST and produces a ResolvedProgram ready for code generation.

pub mod field_types;
pub mod behavior;
pub mod schema;

use crate::ast::*;
use crate::error::SpokeError;
use field_types::FieldType;

// ─── Resolved Types ───────────────────────────────────────────────────────────

/// A fully resolved field — type inferred, constraints determined
#[derive(Debug, Clone)]
pub struct ResolvedField {
    pub name: String,           // snake_case
    pub field_type: FieldType,
    pub required: bool,
    pub unique: bool,
    pub private: bool,          // never exposed in API responses
    pub indexed: bool,
    pub default: Option<String>,
    pub validation: Vec<String>,
}

/// A fully resolved entity — fields typed, relations concrete
#[derive(Debug, Clone)]
pub struct ResolvedEntity {
    pub name: String,           // PascalCase in output
    pub table: String,          // snake_case in DB
    pub fields: Vec<ResolvedField>,
    pub has_timestamps: bool,   // createdAt / updatedAt — always true
    pub has_soft_delete: bool,
}

/// A resolved action — what code to generate
#[derive(Debug, Clone)]
pub struct ResolvedAction {
    pub actor: Actor,
    pub kind: ActionKind,
    pub entity: String,
    pub ownership: OwnershipKind,
    pub guard: Option<String>,  // middleware expression
}

#[derive(Debug, Clone)]
pub enum ActionKind {
    Create,
    Read,
    Update,
    Delete,
    List,
    Upload,
    Custom(String),
}

#[derive(Debug, Clone)]
pub enum OwnershipKind {
    Own,    // only their own records
    Any,    // any record (admin)
    All,    // public read
}

/// A resolved page
#[derive(Debug, Clone)]
pub struct ResolvedPage {
    pub name: String,
    pub route: String,          // /tasks, /profile etc
    pub access: AccessLevel,
    pub queries: Vec<ResolvedQuery>,
    pub mutations: Vec<String>,
}

/// A resolved data query
#[derive(Debug, Clone)]
pub struct ResolvedQuery {
    pub entity: String,
    pub filter: Option<String>,
    pub order_by: Option<(String, String)>, // (field, ASC|DESC)
    pub limit: Option<u32>,
    pub ownership: OwnershipKind,
}

/// Auth configuration
#[derive(Debug, Clone)]
pub struct ResolvedAuth {
    pub email_password: bool,
    pub magic_link: bool,
    pub oauth_providers: Vec<String>,
    pub session_days: u32,
    pub login_redirect: String,
    pub logout_redirect: String,
}

/// Full resolved program — input to codegen
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub app_name: String,
    pub auth: Option<ResolvedAuth>,
    pub entities: Vec<ResolvedEntity>,
    pub actions: Vec<ResolvedAction>,
    pub pages: Vec<ResolvedPage>,
    pub triggers: Vec<ResolvedTrigger>,
}

#[derive(Debug, Clone)]
pub struct ResolvedTrigger {
    pub event: String,
    pub actions: Vec<String>,
}

// ─── Main Resolver ────────────────────────────────────────────────────────────

pub struct RuleEngine;

impl RuleEngine {
    pub fn resolve(program: &Program) -> Result<ResolvedProgram, SpokeError> {
        let mut resolved = ResolvedProgram {
            app_name: String::new(),
            auth: None,
            entities: Vec::new(),
            actions: Vec::new(),
            pages: Vec::new(),
            triggers: Vec::new(),
        };

        for decl in &program.declarations {
            match decl {
                Declaration::App(app) => Self::resolve_app(app, &mut resolved)?,
                Declaration::Page(page) => {
                    resolved.pages.push(Self::resolve_page(page));
                }
                _ => {}
            }
        }

        Ok(resolved)
    }

    fn resolve_app(app: &AppDecl, out: &mut ResolvedProgram) -> Result<(), SpokeError> {
        out.app_name = app.name.clone();

        // Auth
        if let Some(auth) = &app.auth {
            out.auth = Some(Self::resolve_auth(auth));
        }

        // Entities
        for entity in &app.data {
            out.entities.push(Self::resolve_entity(entity));
        }

        // Behavior
        for stmt in &app.behavior {
            match stmt {
                BehaviorStmt::Action(action) => {
                    out.actions.push(Self::resolve_action(action));
                }
                BehaviorStmt::Trigger(trigger) => {
                    out.triggers.push(Self::resolve_trigger(trigger));
                }
                _ => {}
            }
        }

        // Pages
        for page in &app.pages {
            out.pages.push(Self::resolve_page(page));
        }

        Ok(())
    }

    // ─── Auth Resolution ──────────────────────────────────────────────────────

    fn resolve_auth(auth: &AuthBlock) -> ResolvedAuth {
        let mut resolved = ResolvedAuth {
            email_password: false,
            magic_link: false,
            oauth_providers: Vec::new(),
            session_days: 30,
            login_redirect: "/dashboard".to_string(),
            logout_redirect: "/login".to_string(),
        };

        for method in &auth.methods {
            match method {
                AuthMethod::EmailPassword { .. } => resolved.email_password = true,
                AuthMethod::MagicLink           => resolved.magic_link = true,
                AuthMethod::OAuth { provider }  => {
                    resolved.oauth_providers.push(format!("{:?}", provider).to_lowercase());
                }
            }
        }

        if let Some(session) = &auth.session {
            resolved.session_days = session.duration_days;
        }

        for redirect in &auth.redirects {
            match redirect.event {
                AuthEvent::Login  | AuthEvent::Signup => {
                    resolved.login_redirect = redirect.path.clone();
                }
                AuthEvent::Logout => {
                    resolved.logout_redirect = redirect.path.clone();
                }
            }
        }

        resolved
    }

    // ─── Entity / Field Resolution ────────────────────────────────────────────

    fn resolve_entity(entity: &EntityDecl) -> ResolvedEntity {
        let fields = entity.fields.iter()
            .map(|f| Self::resolve_field(f))
            .collect();

        ResolvedEntity {
            name: to_pascal_case(&entity.name),
            table: to_snake_case(&entity.name),
            fields,
            has_timestamps: true,
            has_soft_delete: false,
        }
    }

    fn resolve_field(field: &FieldDecl) -> ResolvedField {
        let field_type = field_types::infer_type(&field.name);
        let required = !field.modifiers.contains(&FieldModifier::Optional);
        let unique   = field.modifiers.contains(&FieldModifier::Unique);
        let private  = field.modifiers.contains(&FieldModifier::Private);
        let indexed  = field.modifiers.contains(&FieldModifier::Indexed) || unique;

        // Auto-add validations based on type
        let mut validation = Vec::new();
        match &field_type {
            FieldType::Email    => validation.push("email_format".to_string()),
            FieldType::Url      => validation.push("url_format".to_string()),
            FieldType::Password => {
                validation.push("min_length:8".to_string());
                validation.push("hashed".to_string());
            }
            FieldType::Integer { min: Some(m), .. } => {
                validation.push(format!("min:{}", m));
            }
            _ => {}
        }

        // Auto-default for certain fields
        let default = match &field_type {
            FieldType::Boolean => Some("false".to_string()),
            FieldType::Status  => Some("\"pending\"".to_string()),
            _                  => None,
        };

        ResolvedField {
            name: to_snake_case(&field.name),
            field_type,
            required,
            unique,
            private,
            indexed,
            default,
            validation,
        }
    }

    // ─── Behavior Resolution ──────────────────────────────────────────────────

    fn resolve_action(action: &ActionStmt) -> ResolvedAction {
        let kind = match &action.verb {
            Verb::Create            => ActionKind::Create,
            Verb::Edit | Verb::Update => ActionKind::Update,
            Verb::Delete            => ActionKind::Delete,
            Verb::View              => ActionKind::Read,
            Verb::Upload            => ActionKind::Upload,
            Verb::Mark              => ActionKind::Update,
            Verb::Custom(s)         => ActionKind::Custom(s.clone()),
            _                       => ActionKind::Read,
        };

        let (entity, ownership) = match &action.object {
            Object::TheirOwn(e) => (e.clone(), OwnershipKind::Own),
            Object::Any(e)      => (e.clone(), OwnershipKind::Any),
            Object::Entity(e)   => {
                // Infer ownership from actor
                let own = match &action.actor {
                    Actor::Admins => OwnershipKind::Any,
                    Actor::Anyone => OwnershipKind::All,
                    _             => OwnershipKind::Own,
                };
                (e.clone(), own)
            }
            Object::ByVoice(e)  => (e.clone(), OwnershipKind::Own),
        };

        // Build guard expression
        let guard = behavior::build_guard(&action.actor, &ownership);

        ResolvedAction {
            actor: action.actor.clone(),
            kind,
            entity: to_singular(&entity),
            ownership,
            guard,
        }
    }

    fn resolve_trigger(trigger: &TriggerStmt) -> ResolvedTrigger {
        let event = match &trigger.event {
            TriggerEvent::UserSignsUp          => "user.created".to_string(),
            TriggerEvent::UserLogsIn           => "user.login".to_string(),
            TriggerEvent::EntityCreated(e)     => format!("{}.created", e),
            TriggerEvent::EntityUpdated(e)     => format!("{}.updated", e),
            TriggerEvent::EntityDeleted(e)     => format!("{}.deleted", e),
            TriggerEvent::Custom(s)            => s.clone(),
            _                                  => "unknown".to_string(),
        };

        let actions = trigger.actions.iter().map(|a| match a {
            ActionBody::Notify { target, message } =>
                format!("notify({}, {:?})", target, message),
            ActionBody::SendEmail { to, subject } =>
                format!("sendEmail({}, {:?})", to, subject),
            ActionBody::Raw(s) => s.clone(),
            _ => String::new(),
        }).collect();

        ResolvedTrigger { event, actions }
    }

    // ─── Page Resolution ──────────────────────────────────────────────────────

    fn resolve_page(page: &PageDecl) -> ResolvedPage {
        let route = page_name_to_route(&page.name);
        let mut queries = Vec::new();

        for stmt in &page.body {
            if let PageStmt::Show(show) = stmt {
                // Only resolve as data query if it's actually fetching entity data
                // Skip display rules like "show deadline in red if..."
                match &show.data {
                    DataExpr::Entity(e) => {
                        // Skip if it looks like a field name rather than entity
                        // Heuristic: entity names won't be common field names
                        let field_names = ["deadline", "title", "price", "name",
                            "status", "date", "email", "description"];
                        if !field_names.contains(&e.as_str()) {
                            queries.push(Self::resolve_query(show));
                        }
                    }
                    DataExpr::UserOwned(_) | DataExpr::All(_) |
                    DataExpr::Latest(_, _) | DataExpr::Top(_, _, _) => {
                        queries.push(Self::resolve_query(show));
                    }
                    _ => {}
                }
            }
        }

        ResolvedPage {
            name: page.name.clone(),
            route,
            access: page.access.clone(),
            queries,
            mutations: Vec::new(),
        }
    }

    fn resolve_query(show: &ShowStmt) -> ResolvedQuery {
        let (entity, ownership) = match &show.data {
            DataExpr::UserOwned(e)     => (e.clone(), OwnershipKind::Own),
            DataExpr::All(e)           => (e.clone(), OwnershipKind::All),
            DataExpr::Latest(_, e)     => (e.clone(), OwnershipKind::All),
            DataExpr::Top(_, e, _)     => (e.clone(), OwnershipKind::All),
            DataExpr::Filtered(e, _)   => (e.clone(), OwnershipKind::Own),
            DataExpr::Entity(e)        => (e.clone(), OwnershipKind::All),
            DataExpr::Component(e)     => (e.clone(), OwnershipKind::All),
        };

        let limit = match &show.data {
            DataExpr::Latest(n, _) | DataExpr::Top(n, _, _) => Some(*n),
            _ => None,
        };

        let order_by = show.options.iter().find_map(|opt| {
            if let DisplayOption::SortedBy(field, dir) = opt {
                Some((field.clone(), match dir {
                    SortDir::Asc  => "ASC".to_string(),
                    SortDir::Desc => "DESC".to_string(),
                }))
            } else { None }
        });

        ResolvedQuery {
            entity: to_singular(&entity),
            filter: None,
            order_by,
            limit,
            ownership,
        }
    }
}

// ─── String Utilities ─────────────────────────────────────────────────────────

pub fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-' || c == ' ')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None    => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

pub fn to_snake_case(s: &str) -> String {
    s.replace('-', "_").replace(' ', "_").to_lowercase()
}

pub fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    let mut chars = pascal.chars();
    match chars.next() {
        None    => String::new(),
        Some(f) => f.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

fn to_singular(s: &str) -> String {
    // Simple English singularization
    if s.ends_with("ies") {
        format!("{}y", &s[..s.len()-3])
    } else if s.ends_with('s') && !s.ends_with("ss") && !s.ends_with("us") {
        s[..s.len()-1].to_string()
    } else {
        s.to_string()
    }
}

fn page_name_to_route(name: &str) -> String {
    format!("/{}", name.to_lowercase().replace(' ', "-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pascal_case() {
        assert_eq!(to_pascal_case("task"),         "Task");
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
        assert_eq!(to_pascal_case("stock_count"),  "StockCount");
    }

    #[test]
    fn test_singular() {
        assert_eq!(to_singular("tasks"),    "task");
        assert_eq!(to_singular("users"),    "user");
        assert_eq!(to_singular("stories"),  "story");
        assert_eq!(to_singular("status"),   "status");
    }
}
