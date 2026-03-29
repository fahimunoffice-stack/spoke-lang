use crate::rules::ResolvedProgram;

pub fn generate(program: &ResolvedProgram) -> String {
    let name = program.app_name.to_lowercase().replace(' ', "-");
    let has_auth = program.auth.is_some();
    let has_oauth = program.auth.as_ref().map(|a| !a.oauth_providers.is_empty()).unwrap_or(false);

    let mut deps = String::from(r#"    "next": "^15.0.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "@prisma/client": "^5.22.0""#);

    if has_auth {
        deps.push_str(",\n    \"next-auth\": \"^4.24.0\",\n    \"bcryptjs\": \"^2.4.3\"");
    }

    format!(r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "private": true,
  "scripts": {{
    "dev":          "next dev",
    "build":        "next build",
    "start":        "next start",
    "db:push":      "prisma db push",
    "db:studio":    "prisma studio",
    "db:generate":  "prisma generate"
  }},
  "dependencies": {{
{deps}
  }},
  "devDependencies": {{
    "typescript":          "^5.6.0",
    "@types/node":         "^22.0.0",
    "@types/react":        "^18.3.0",
    "@types/react-dom":    "^18.3.0",
    "@types/bcryptjs":     "^2.4.6",
    "tailwindcss":         "^3.4.0",
    "autoprefixer":        "^10.4.0",
    "postcss":             "^8.4.0",
    "prisma":              "^5.22.0"
  }}
}}
"#)
}
