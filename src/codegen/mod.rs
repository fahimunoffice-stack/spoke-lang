pub mod web;
pub mod nextauth;
pub mod api_routes;
pub mod pages;
pub mod package;

use crate::rules::ResolvedProgram;
use std::path::Path;

pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

pub struct WebTarget;

impl WebTarget {
    pub fn generate(program: &ResolvedProgram, out_dir: &Path) -> Vec<GeneratedFile> {
        let mut files = Vec::new();

        files.push(gf("prisma/schema.prisma", crate::rules::schema::generate_prisma(program)));
        files.push(gf("types/index.ts",       crate::rules::schema::generate_typescript_types(program)));
        files.push(gf("package.json",          package::generate(program)));
        files.push(gf("next.config.ts",        web::next_config()));
        files.push(gf("tsconfig.json",         web::tsconfig()));
        files.push(gf("tailwind.config.ts",    web::tailwind_config()));
        files.push(gf("lib/db.ts",             web::prisma_client()));
        files.push(gf("app/layout.tsx",        web::root_layout(&program.app_name)));
        files.push(gf("app/globals.css",       web::globals_css()));
        files.push(gf("app/page.tsx",          web::root_page(program)));
        files.push(gf(".env.example",          web::env_example(program)));

        if program.auth.is_some() {
            files.push(gf("lib/auth.ts",             nextauth::generate(program)));
            files.push(gf("app/login/page.tsx",      web::login_page(program)));
            files.push(gf("app/signup/page.tsx",     web::signup_page(program)));
            files.push(gf("app/api/auth/[...nextauth]/route.ts", nextauth::route_handler()));
        }

        for entity in &program.entities {
            let slug = entity.table.clone() + "s";
            let acts: Vec<_> = program.actions.iter()
                .filter(|a| a.entity.to_lowercase() == entity.table)
                .collect();
            files.push(gf(
                &format!("app/api/{}/route.ts", slug),
                api_routes::generate_list_create(entity, &acts),
            ));
            files.push(gf(
                &format!("app/api/{}/[id]/route.ts", slug),
                api_routes::generate_get_update_delete(entity, &acts),
            ));
        }

        for page in &program.pages {
            let route = page.name.to_lowercase().replace(' ', "-");
            files.push(gf(
                &format!("app/{}/page.tsx", route),
                pages::generate_page(page, program),
            ));
        }

        // Write to disk
        for f in &files {
            let path = out_dir.join(&f.path);
            if let Some(p) = path.parent() { std::fs::create_dir_all(p).ok(); }
            std::fs::write(&path, &f.content).ok();
        }

        files
    }
}

fn gf(path: &str, content: String) -> GeneratedFile {
    GeneratedFile { path: path.to_string(), content }
}
