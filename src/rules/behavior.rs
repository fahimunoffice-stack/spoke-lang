// src/rules/behavior.rs
// Resolves behavior intents into concrete guard expressions and middleware.

use crate::ast::Actor;
use crate::rules::OwnershipKind;

/// Build a guard expression for an action.
/// This becomes middleware in the generated code.
pub fn build_guard(actor: &Actor, ownership: &OwnershipKind) -> Option<String> {
    match actor {
        Actor::Anyone => None, // public, no guard

        Actor::Users | Actor::OnlyLoggedIn => {
            match ownership {
                OwnershipKind::Own => Some("requireAuth() && requireOwnership()".to_string()),
                OwnershipKind::Any => Some("requireAuth()".to_string()),
                OwnershipKind::All => Some("requireAuth()".to_string()),
            }
        }

        Actor::Admins => Some("requireAdmin()".to_string()),

        Actor::Role(role) => Some(format!("requireRole(\"{}\")", role)),
    }
}

/// Map an actor to a Prisma/SQL where clause for ownership
pub fn ownership_where(ownership: &OwnershipKind, user_id_field: &str) -> Option<String> {
    match ownership {
        OwnershipKind::Own => Some(format!("{{ {}: session.user.id }}", user_id_field)),
        OwnershipKind::Any | OwnershipKind::All => None,
    }
}

/// Generate Next.js middleware for a route
pub fn nextjs_middleware(actor: &Actor) -> String {
    match actor {
        Actor::Anyone => String::new(),
        Actor::Users | Actor::OnlyLoggedIn => {
            r#"const session = await getServerSession(authOptions);
  if (!session) return NextResponse.redirect(new URL('/login', req.url));"#.to_string()
        }
        Actor::Admins => {
            r#"const session = await getServerSession(authOptions);
  if (!session) return NextResponse.redirect(new URL('/login', req.url));
  if (session.user.role !== 'admin') return NextResponse.json({ error: 'Forbidden' }, { status: 403 });"#.to_string()
        }
        Actor::Role(role) => {
            format!(r#"const session = await getServerSession(authOptions);
  if (!session) return NextResponse.redirect(new URL('/login', req.url));
  if (session.user.role !== '{}') return NextResponse.json({{ error: 'Forbidden' }}, {{ status: 403 }});"#, role)
        }
    }
}

/// Generate Flutter auth check for a screen
pub fn flutter_auth_check(actor: &Actor) -> String {
    match actor {
        Actor::Anyone => String::new(),
        Actor::Users | Actor::OnlyLoggedIn => {
            r#"if (!context.read<AuthProvider>().isLoggedIn) {
      Navigator.pushReplacementNamed(context, '/login');
      return;
    }"#.to_string()
        }
        Actor::Admins => {
            r#"if (!context.read<AuthProvider>().isAdmin) {
      Navigator.pushReplacementNamed(context, '/');
      return;
    }"#.to_string()
        }
        _ => String::new(),
    }
}
