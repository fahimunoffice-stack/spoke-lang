// src/parser/mod.rs
// Recursive descent parser.
// Converts a flat list of SpannedTokens into the Intent AST.

use crate::lexer::token::{Token, SpannedToken};
use crate::ast::*;
use crate::error::SpokeError;

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        // Filter out bare newlines — we only care about Indent/Dedent for structure
        let tokens = tokens
            .into_iter()
            .filter(|t| t.token != Token::NewLine)
            .collect();
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, SpokeError> {
        let mut declarations = Vec::new();

        while !self.is_eof() {
            let decl = self.parse_declaration()?;
            declarations.push(decl);
        }

        Ok(Program { declarations })
    }

    // ─── Declarations ─────────────────────────────────────────────────────────

    fn parse_declaration(&mut self) -> Result<Declaration, SpokeError> {
        match self.peek_token() {
            Token::App       => Ok(Declaration::App(self.parse_app()?)),
            Token::Page      => Ok(Declaration::Page(self.parse_page()?)),
            Token::Api       => Ok(Declaration::Api(self.parse_api()?)),
            Token::Service   => Ok(Declaration::Service(self.parse_service()?)),
            Token::Component => Ok(Declaration::Component(self.parse_component()?)),
            other => Err(SpokeError::UnexpectedToken {
                expected: "app, page, api, service, or component".to_string(),
                found: format!("{:?}", other),
                line: self.current_line(),
            }),
        }
    }

    // ─── App ──────────────────────────────────────────────────────────────────

    fn parse_app(&mut self) -> Result<AppDecl, SpokeError> {
        self.expect(Token::App)?;
        let name = self.expect_string()?;
        self.expect(Token::Indent)?;

        let mut auth = None;
        let mut data = Vec::new();
        let mut behavior = Vec::new();
        let mut pages = Vec::new();
        let mut navigation = None;
        let mut services = Vec::new();

        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            match self.peek_token() {
                Token::Auth      => auth = Some(self.parse_auth_block()?),
                Token::Data      => data = self.parse_data_block()?,
                Token::Behavior  => behavior = self.parse_behavior_block()?,
                Token::Pages     => pages = self.parse_pages_block()?,
                Token::Navigation=> navigation = Some(self.parse_navigation()?),
                Token::Service   => services.push(self.parse_service()?),
                _ => { self.advance(); } // skip unknown tokens
            }
        }

        self.expect(Token::Dedent)?;

        Ok(AppDecl { name, auth, data, behavior, pages, navigation, services })
    }

    // ─── Auth ─────────────────────────────────────────────────────────────────

    fn parse_auth_block(&mut self) -> Result<AuthBlock, SpokeError> {
        self.expect(Token::Auth)?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;

        let mut methods = Vec::new();
        let mut session = None;
        let mut redirects = Vec::new();

        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            match self.peek_token() {
                Token::Login => {
                    self.advance(); // login
                    self.expect(Token::With)?;
                    let fields = self.parse_word_list()?;
                    methods.push(AuthMethod::EmailPassword { signup_fields: fields });
                }
                Token::Signup => {
                    self.advance(); // signup
                    self.expect(Token::With)?;
                    let fields = self.parse_word_list()?;
                    methods.push(AuthMethod::EmailPassword { signup_fields: fields });
                }
                Token::Also => {
                    self.advance(); // also
                    self.expect(Token::Allow)?;
                    let provider = self.parse_oauth_provider()?;
                    methods.push(AuthMethod::OAuth { provider });
                }
                Token::Magic => {
                    self.advance(); // magic
                    // "magic link login" — consume remaining words
                    while matches!(self.peek_token(), Token::Link | Token::Login | Token::Identifier(_)) {
                        self.advance();
                    }
                    methods.push(AuthMethod::MagicLink);
                }
                Token::Remember => {
                    self.advance(); // remember
                    self.advance(); // login
                    self.advance(); // for
                    let n = self.expect_number()? as u32;
                    self.advance(); // days
                    session = Some(SessionConfig { duration_days: n });
                }
                Token::After => {
                    self.advance(); // after
                    let event = match self.peek_token() {
                        Token::Login  => { self.advance(); AuthEvent::Login }
                        Token::Logout => { self.advance(); AuthEvent::Logout }
                        Token::Signup => { self.advance(); AuthEvent::Signup }
                        _ => AuthEvent::Login,
                    };
                    self.advance(); // go
                    self.advance(); // to
                    let path = self.expect_string_or_path()?;
                    redirects.push(RedirectConfig { event, path });
                }
                _ => { self.advance(); }
            }
        }

        self.expect(Token::Dedent)?;
        Ok(AuthBlock { methods, session, redirects })
    }

    fn parse_oauth_provider(&mut self) -> Result<OAuthProvider, SpokeError> {
        match self.peek_token() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                self.advance(); // "login"
                Ok(match name.as_str() {
                    "Google"   => OAuthProvider::Google,
                    "GitHub"   => OAuthProvider::GitHub,
                    "Facebook" => OAuthProvider::Facebook,
                    "Apple"    => OAuthProvider::Apple,
                    other      => OAuthProvider::Custom(other.to_string()),
                })
            }
            _ => Ok(OAuthProvider::Custom("unknown".to_string())),
        }
    }

    // ─── Data ─────────────────────────────────────────────────────────────────

    fn parse_data_block(&mut self) -> Result<Vec<EntityDecl>, SpokeError> {
        self.expect(Token::Data)?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;

        let mut entities = Vec::new();

        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            entities.push(self.parse_entity()?);
        }

        self.expect(Token::Dedent)?;
        Ok(entities)
    }

    fn parse_entity(&mut self) -> Result<EntityDecl, SpokeError> {
        let name = self.expect_identifier()?;
        self.expect(Token::Has)?;

        let mut fields = Vec::new();
        let mut relations = Vec::new();

        if matches!(self.peek_token(), Token::Colon) {
            // Multiline: "task has:"
            self.advance(); // :
            self.expect(Token::Indent)?;
            while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
                fields.push(self.parse_field()?);
            }
            self.expect(Token::Dedent)?;
        } else {
            // Inline: "task has title, deadline, and status"
            fields = self.parse_field_list_inline()?;
        }

        Ok(EntityDecl { name, fields, relations })
    }

    fn parse_field(&mut self) -> Result<FieldDecl, SpokeError> {
        let start_line = self.current_line();
        let mut name = self.expect_identifier()?;

        // Absorb multi-word field names on SAME LINE only: "stock count" → "stock_count"
        loop {
            if self.current_line() != start_line { break; }
            match self.peek_token() {
                Token::Identifier(next) => {
                    name = format!("{}_{}", name, next.clone());
                    self.advance();
                }
                _ => break,
            }
        }

        let mut modifiers = Vec::new();
        loop {
            match self.peek_token() {
                Token::Unique    => { self.advance(); modifiers.push(FieldModifier::Unique); }
                Token::Optional  => { self.advance(); modifiers.push(FieldModifier::Optional); }
                Token::Private   => { self.advance(); modifiers.push(FieldModifier::Private); }
                Token::Required  => { self.advance(); modifiers.push(FieldModifier::Required); }
                Token::Indexed   => { self.advance(); modifiers.push(FieldModifier::Indexed); }
                _ => break,
            }
        }

        Ok(FieldDecl { name, modifiers })
    }

    fn parse_field_list_inline(&mut self) -> Result<Vec<FieldDecl>, SpokeError> {
        let mut fields = Vec::new();
        fields.push(self.parse_field()?);

        loop {
            // Consume all separators: "," or "and" or ", and"
            let mut got_sep = false;
            while matches!(self.peek_token(), Token::Comma | Token::And) {
                self.advance();
                got_sep = true;
            }
            if !got_sep { break; }

            // Next must be a field name
            match self.peek_token() {
                Token::Identifier(_) | Token::Login | Token::Logout |
                Token::User | Token::Users | Token::Admin | Token::Mark |
                Token::Record | Token::Search | Token::Send | Token::Store => {
                    fields.push(self.parse_field()?);
                }
                _ => break,
            }
        }

        Ok(fields)
    }

    // ─── Behavior ─────────────────────────────────────────────────────────────

    fn parse_behavior_block(&mut self) -> Result<Vec<BehaviorStmt>, SpokeError> {
        self.expect(Token::Behavior)?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;

        let mut stmts = Vec::new();

        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            stmts.push(self.parse_behavior_stmt()?);
        }

        self.expect(Token::Dedent)?;
        Ok(stmts)
    }

    fn parse_behavior_stmt(&mut self) -> Result<BehaviorStmt, SpokeError> {
        match self.peek_token() {
            Token::When  => Ok(BehaviorStmt::Trigger(self.parse_trigger()?)),
            Token::Every => Ok(BehaviorStmt::Schedule(self.parse_schedule()?)),
            _            => Ok(BehaviorStmt::Action(self.parse_action()?)),
        }
    }

    fn parse_action(&mut self) -> Result<ActionStmt, SpokeError> {
        let actor = self.parse_actor()?;
        self.expect(Token::Can)?;
        let verb = self.parse_verb()?;
        let object = self.parse_object()?;
        let condition = self.try_parse_condition()?;
        // Drain any remaining tokens on this logical line (e.g. "as complete", "by voice")
        self.drain_line();
        Ok(ActionStmt { actor, verb, object, condition })
    }

    /// Consume tokens until we hit a structural boundary (Indent/Dedent/Eof)
    fn drain_line(&mut self) {
        while !matches!(self.peek_token(), Token::Indent | Token::Dedent | Token::Eof) {
            // Stop if next token looks like the start of a new statement
            match self.peek_token() {
                Token::Users | Token::Admins | Token::Anyone |
                Token::When  | Token::Every  | Token::Page   |
                Token::Show  | Token::Form   | Token::Require => break,
                _ => { self.advance(); }
            }
        }
    }

    fn parse_actor(&mut self) -> Result<Actor, SpokeError> {
        match self.peek_token() {
            Token::Users  => { self.advance(); Ok(Actor::Users) }
            Token::User   => { self.advance(); Ok(Actor::Users) }
            Token::Admins => { self.advance(); Ok(Actor::Admins) }
            Token::Admin  => { self.advance(); Ok(Actor::Admins) }
            Token::Anyone => { self.advance(); Ok(Actor::Anyone) }
            Token::Only   => {
                self.advance(); // only
                self.advance(); // logged-in
                self.advance(); // users
                Ok(Actor::OnlyLoggedIn)
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Actor::Role(name))
            }
            other => Err(SpokeError::UnexpectedToken {
                expected: "actor (users, admins, anyone)".to_string(),
                found: format!("{:?}", other),
                line: self.current_line(),
            }),
        }
    }

    fn parse_verb(&mut self) -> Result<Verb, SpokeError> {
        let verb = match self.peek_token() {
            Token::Create => Verb::Create,
            Token::Edit   => Verb::Edit,
            Token::Update => Verb::Update,
            Token::Delete => Verb::Delete,
            Token::View   => Verb::View,
            Token::Upload => Verb::Upload,
            Token::Send   => Verb::Send,
            Token::Mark   => Verb::Mark,
            Token::Search => Verb::Search,
            Token::Record => Verb::Record,
            Token::Login  => Verb::Login,
            Token::Logout => Verb::Logout,
            Token::Signup => Verb::Signup,
            Token::Identifier(name) => Verb::Custom(name.clone()),
            other => return Err(SpokeError::UnexpectedToken {
                expected: "action verb (create, edit, delete...)".to_string(),
                found: format!("{:?}", other),
                line: self.current_line(),
            }),
        };
        self.advance();
        Ok(verb)
    }

    fn parse_object(&mut self) -> Result<Object, SpokeError> {
        match self.peek_token() {
            Token::Their => {
                self.advance(); // their
                let _own = matches!(self.peek_token(), Token::Own);
                if _own { self.advance(); }
                let name = self.expect_identifier()?;
                Ok(Object::TheirOwn(name))
            }
            Token::Any => {
                self.advance(); // any
                let name = self.expect_identifier()?;
                Ok(Object::Any(name))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                // check for "by voice" etc
                if matches!(self.peek_token(), Token::By) {
                    self.advance(); // by
                    let how = self.expect_identifier()?;
                    Ok(Object::ByVoice(format!("{} by {}", name, how)))
                } else {
                    Ok(Object::Entity(name))
                }
            }
            _ => Ok(Object::Entity("unknown".to_string())),
        }
    }

    fn try_parse_condition(&mut self) -> Result<Option<Condition>, SpokeError> {
        match self.peek_token() {
            Token::If | Token::Unless => {
                self.advance();
                Ok(Some(self.parse_condition_expr()?))
            }
            _ => Ok(None),
        }
    }

    fn parse_condition_expr(&mut self) -> Result<Condition, SpokeError> {
        // "they own the task"
        // "they are admin"
        // "field is value"
        let cond = match self.peek_token() {
            Token::Identifier(_) | Token::User => {
                let subject = self.expect_identifier().unwrap_or("they".to_string());
                match self.peek_token() {
                    Token::Is | Token::Are => {
                        self.advance();
                        let role = self.expect_identifier()?;
                        Condition::ActorIs(role)
                    }
                    Token::Own => {
                        self.advance(); // own
                        self.advance(); // the
                        let entity = self.expect_identifier()?;
                        Condition::ActorOwns(entity)
                    }
                    _ => Condition::ActorIs(subject),
                }
            }
            _ => {
                self.advance();
                Condition::ActorOwns("resource".to_string())
            }
        };

        // Check for "and" / "or"
        match self.peek_token() {
            Token::And => {
                self.advance();
                let right = self.parse_condition_expr()?;
                Ok(Condition::And(Box::new(cond), Box::new(right)))
            }
            Token::Or => {
                self.advance();
                let right = self.parse_condition_expr()?;
                Ok(Condition::Or(Box::new(cond), Box::new(right)))
            }
            _ => Ok(cond),
        }
    }

    fn parse_trigger(&mut self) -> Result<TriggerStmt, SpokeError> {
        self.expect(Token::When)?;
        let event = self.parse_trigger_event()?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;

        let mut actions = Vec::new();
        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            actions.push(self.parse_action_body()?);
        }

        self.expect(Token::Dedent)?;
        Ok(TriggerStmt { event, actions })
    }

    fn parse_trigger_event(&mut self) -> Result<TriggerEvent, SpokeError> {
        match self.peek_token() {
            Token::User | Token::Users => {
                self.advance();
                match self.peek_token() {
                    Token::Signup | Token::Identifier(_) if self.peek_str_contains("sign") => {
                        self.drain_line(); Ok(TriggerEvent::UserSignsUp)
                    }
                    Token::Login  => { self.advance(); Ok(TriggerEvent::UserLogsIn) }
                    Token::Logout => { self.advance(); Ok(TriggerEvent::UserLogsOut) }
                    _ => { self.drain_line(); Ok(TriggerEvent::UserSignsUp) }
                }
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                // "order is created", "product stock reaches 0"
                // consume remaining words before the colon
                let mut event_words = vec![name.clone()];
                while !matches!(self.peek_token(), Token::Colon | Token::Indent | Token::Dedent | Token::Eof) {
                    if let Token::Identifier(w) = self.peek_token() {
                        event_words.push(w.clone());
                    }
                    self.advance();
                }
                let event_str = event_words.join(" ");
                // Map common patterns
                if event_str.contains("created") || event_str.contains("is created") {
                    Ok(TriggerEvent::EntityCreated(name))
                } else if event_str.contains("updated") {
                    Ok(TriggerEvent::EntityUpdated(name))
                } else if event_str.contains("deleted") {
                    Ok(TriggerEvent::EntityDeleted(name))
                } else {
                    Ok(TriggerEvent::Custom(event_str))
                }
            }
            _ => { self.advance(); Ok(TriggerEvent::Custom("event".to_string())) }
        }
    }

    fn peek_str_contains(&self, s: &str) -> bool {
        if let Token::Identifier(name) = self.peek_token() {
            name.contains(s)
        } else { false }
    }

    fn parse_schedule(&mut self) -> Result<ScheduleStmt, SpokeError> {
        self.expect(Token::Every)?;
        let interval = match self.peek_token() {
            Token::Hour => { self.advance(); ScheduleInterval::EveryHour }
            Token::Day  => {
                self.advance();
                let time = if matches!(self.peek_token(), Token::At) {
                    self.advance();
                    Some(self.expect_string_or_path()?)
                } else {
                    None
                };
                ScheduleInterval::EveryDay { time }
            }
            _ => ScheduleInterval::EveryHour,
        };
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;

        let mut actions = Vec::new();
        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            actions.push(self.parse_action_body()?);
        }

        self.expect(Token::Dedent)?;
        Ok(ScheduleStmt { interval, actions })
    }

    fn parse_action_body(&mut self) -> Result<ActionBody, SpokeError> {
        match self.peek_token() {
            Token::Notify => {
                self.advance();
                let target = self.expect_identifier().unwrap_or("user".to_string());
                let msg = self.expect_string().unwrap_or_default();
                Ok(ActionBody::Notify { target, message: msg })
            }
            Token::Send => {
                self.advance();
                // "send email to user"
                let kind = self.expect_identifier().unwrap_or("email".to_string());
                self.advance(); // "to"
                let to = self.expect_identifier().unwrap_or("user".to_string());
                Ok(ActionBody::SendEmail { to, subject: None })
            }
            _ => {
                // Collect rest of line as raw action
                let mut parts = Vec::new();
                while !matches!(self.peek_token(), Token::Eof | Token::Dedent) {
                    parts.push(format!("{:?}", self.peek_token()));
                    self.advance();
                }
                Ok(ActionBody::Raw(parts.join(" ")))
            }
        }
    }

    // ─── Pages ────────────────────────────────────────────────────────────────

    fn parse_pages_block(&mut self) -> Result<Vec<PageDecl>, SpokeError> {
        self.expect(Token::Pages)?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;

        let mut pages = Vec::new();
        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            pages.push(self.parse_page()?);
        }

        self.expect(Token::Dedent)?;
        Ok(pages)
    }

    fn parse_page(&mut self) -> Result<PageDecl, SpokeError> {
        self.expect(Token::Page)?;
        let name = self.expect_string()?;
        self.expect(Token::Indent)?;

        let mut access = AccessLevel::Public;
        let mut body = Vec::new();

        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            match self.peek_token() {
                Token::Require => {
                    self.advance();
                    access = match self.peek_token() {
                        Token::Login => { self.advance(); AccessLevel::RequireLogin }
                        Token::Identifier(role) => {
                            let r = role.clone();
                            self.advance(); self.advance(); // role name + "role"
                            AccessLevel::RequireRole(r)
                        }
                        _ => AccessLevel::RequireLogin,
                    };
                }
                Token::Show => body.push(PageStmt::Show(self.parse_show()?)),
                Token::Form => body.push(PageStmt::Form(self.parse_form()?)),
                _           => { self.advance(); }
            }
        }

        self.expect(Token::Dedent)?;
        Ok(PageDecl { name, access, body })
    }

    fn parse_show(&mut self) -> Result<ShowStmt, SpokeError> {
        self.expect(Token::Show)?;
        let data = self.parse_data_expr()?;
        let mut options = Vec::new();

        // Parse display options: "sorted by deadline", "as cards", "in a grid"
        loop {
            match self.peek_token() {
                Token::Sorted => {
                    self.advance(); // sorted
                    self.advance(); // by
                    let field = self.expect_identifier()?;
                    options.push(DisplayOption::SortedBy(field, SortDir::Asc));
                }
                Token::As => {
                    self.advance(); // as
                    let fmt = self.expect_identifier()?;
                    options.push(DisplayOption::As(fmt));
                }
                Token::In => {
                    self.advance(); // in
                    let layout = self.expect_identifier()?;
                    options.push(DisplayOption::In(layout));
                }
                _ => break,
            }
        }

        Ok(ShowStmt { data, options })
    }

    fn parse_data_expr(&mut self) -> Result<DataExpr, SpokeError> {
        match self.peek_token() {
            Token::User | Token::Users => {
                self.advance();
                // "user's tasks" — we consumed 'user', next should be 's tasks
                // simplified: just grab the entity name
                let name = self.expect_identifier().unwrap_or("items".to_string());
                Ok(DataExpr::UserOwned(name))
            }
            Token::Latest => {
                self.advance();
                let n = self.expect_number()? as u32;
                let name = self.expect_identifier()?;
                Ok(DataExpr::Latest(n, name))
            }
            Token::Top => {
                self.advance();
                let n = self.expect_number()? as u32;
                let name = self.expect_identifier()?;
                self.advance(); // by
                let field = self.expect_identifier()?;
                Ok(DataExpr::Top(n, name, field))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(DataExpr::Entity(name))
            }
            _ => Ok(DataExpr::Entity("items".to_string())),
        }
    }

    fn parse_form(&mut self) -> Result<FormDecl, SpokeError> {
        self.expect(Token::Form)?;
        self.expect(Token::To)?;
        let purpose = self.expect_identifier()?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;

        let mut fields = Vec::new();
        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            let name = self.expect_identifier()?;
            let required = !matches!(self.peek_token(), Token::Optional);
            if matches!(self.peek_token(), Token::Required | Token::Optional) {
                self.advance();
            }
            fields.push(FormField { name, required });
        }

        self.expect(Token::Dedent)?;
        Ok(FormDecl { purpose, fields, on_submit: None })
    }

    fn parse_navigation(&mut self) -> Result<NavigationBlock, SpokeError> {
        self.expect(Token::Navigation)?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;

        let mut items = Vec::new();
        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            match self.peek_token() {
                Token::Logout => {
                    self.advance(); self.advance(); // logout button
                    items.push(NavItem::LogoutButton);
                }
                _ => {
                    let label = self.expect_string()?;
                    self.advance(); // arrow
                    let path = self.expect_string_or_path()?;
                    items.push(NavItem::Link { label, path });
                }
            }
        }

        self.expect(Token::Dedent)?;
        Ok(NavigationBlock { items })
    }

    fn parse_api(&mut self) -> Result<ApiDecl, SpokeError> {
        self.expect(Token::Api)?;
        let version = self.expect_string()?;
        self.expect(Token::Indent)?;

        let mut routes = Vec::new();
        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            let method = match self.peek_token() {
                Token::Get   => { self.advance(); HttpMethod::Get }
                Token::Post  => { self.advance(); HttpMethod::Post }
                Token::Put   => { self.advance(); HttpMethod::Put }
                Token::Patch => { self.advance(); HttpMethod::Patch }
                Token::Delete=> { self.advance(); HttpMethod::Delete }
                _            => { self.advance(); HttpMethod::Get }
            };
            let path = self.expect_string_or_path()?;
            self.advance(); // arrow
            let response = self.collect_to_eol();
            routes.push(ApiRoute { method, path, response, access: None });
        }

        self.expect(Token::Dedent)?;
        Ok(ApiDecl { version, routes })
    }

    fn parse_service(&mut self) -> Result<ServiceDecl, SpokeError> {
        self.expect(Token::Service)?;
        let name = self.expect_string()?;
        self.expect(Token::Indent)?;

        let mut body = Vec::new();
        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            body.push(self.parse_behavior_stmt()?);
        }

        self.expect(Token::Dedent)?;
        Ok(ServiceDecl { name, body })
    }

    fn parse_component(&mut self) -> Result<ComponentDecl, SpokeError> {
        self.expect(Token::Component)?;
        let name = self.expect_string()?;
        self.expect(Token::Indent)?;

        let mut body = Vec::new();
        while !matches!(self.peek_token(), Token::Dedent | Token::Eof) {
            match self.peek_token() {
                Token::Show => body.push(PageStmt::Show(self.parse_show()?)),
                _ => { self.advance(); }
            }
        }

        self.expect(Token::Dedent)?;
        Ok(ComponentDecl { name, body })
    }

    // ─── Helpers ──────────────────────────────────────────────────────────────

    fn parse_word_list(&mut self) -> Result<Vec<String>, SpokeError> {
        let mut words = Vec::new();
        loop {
            match self.peek_token() {
                Token::Identifier(w) => { words.push(w.clone()); self.advance(); }
                Token::And | Token::Comma => { self.advance(); }
                _ => break,
            }
        }
        Ok(words)
    }

    fn collect_to_eol(&mut self) -> String {
        let mut parts = Vec::new();
        while !matches!(self.peek_token(), Token::Eof | Token::Dedent | Token::Indent) {
            if let Token::Identifier(s) = self.peek_token() {
                parts.push(s.clone());
            }
            self.advance();
        }
        parts.join(" ")
    }

    fn expect(&mut self, expected: Token) -> Result<(), SpokeError> {
        let tok = self.peek_token();
        if std::mem::discriminant(&tok) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(SpokeError::UnexpectedToken {
                expected: format!("{:?}", expected),
                found: format!("{:?}", tok),
                line: self.current_line(),
            })
        }
    }

    fn expect_string(&mut self) -> Result<String, SpokeError> {
        match self.peek_token() {
            Token::StringLit(s) => { let s = s.clone(); self.advance(); Ok(s) }
            other => Err(SpokeError::UnexpectedToken {
                expected: "string literal".to_string(),
                found: format!("{:?}", other),
                line: self.current_line(),
            }),
        }
    }

    fn expect_identifier(&mut self) -> Result<String, SpokeError> {
        match self.peek_token() {
            Token::Identifier(s) => { let s = s.clone(); self.advance(); Ok(s) }
            // Common keywords that can act as field names
            Token::Login  => { self.advance(); Ok("login".to_string()) }
            Token::Logout => { self.advance(); Ok("logout".to_string()) }
            Token::User   => { self.advance(); Ok("user".to_string()) }
            Token::Users  => { self.advance(); Ok("users".to_string()) }
            Token::Admin  => { self.advance(); Ok("admin".to_string()) }
            Token::Mark   => { self.advance(); Ok("mark".to_string()) }
            Token::Record => { self.advance(); Ok("record".to_string()) }
            Token::Search => { self.advance(); Ok("search".to_string()) }
            Token::Send   => { self.advance(); Ok("send".to_string()) }
            Token::Store  => { self.advance(); Ok("store".to_string()) }
            other => Err(SpokeError::UnexpectedToken {
                expected: "identifier or name".to_string(),
                found: format!("{:?}", other),
                line: self.current_line(),
            }),
        }
    }

    fn expect_number(&mut self) -> Result<f64, SpokeError> {
        match self.peek_token() {
            Token::NumberLit(n) => { let n = n; self.advance(); Ok(n) }
            other => Err(SpokeError::UnexpectedToken {
                expected: "number".to_string(),
                found: format!("{:?}", other),
                line: self.current_line(),
            }),
        }
    }

    fn expect_string_or_path(&mut self) -> Result<String, SpokeError> {
        match self.peek_token() {
            Token::StringLit(s) => { let s = s.clone(); self.advance(); Ok(s) }
            Token::Identifier(s) => { let s = s.clone(); self.advance(); Ok(format!("/{}", s)) }
            _ => { self.advance(); Ok("/".to_string()) }
        }
    }

    fn peek_token(&self) -> Token {
        self.tokens.get(self.pos)
            .map(|t| t.token.clone())
            .unwrap_or(Token::Eof)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek_token(), Token::Eof)
    }

    fn current_line(&self) -> usize {
        self.tokens.get(self.pos)
            .map(|t| t.line)
            .unwrap_or(0)
    }
}
