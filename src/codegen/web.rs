// src/codegen/web.rs
use crate::rules::{ResolvedProgram};

pub fn next_config() -> String {
r#"import type { NextConfig } from "next";
const nextConfig: NextConfig = { experimental: { serverActions: { allowedOrigins: ["*"] } } };
export default nextConfig;
"#.to_string()
}

pub fn tsconfig() -> String {
r#"{
  "compilerOptions": {
    "target": "ES2017",
    "lib": ["dom", "dom.iterable", "esnext"],
    "allowJs": true,
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "jsx": "preserve",
    "incremental": true,
    "plugins": [{ "name": "next" }],
    "paths": { "@/*": ["./*"] }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules"]
}
"#.to_string()
}

pub fn tailwind_config() -> String {
r#"import type { Config } from "tailwindcss";
const config: Config = {
  content: ["./app/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}"],
  theme: { extend: {} },
  plugins: [],
};
export default config;
"#.to_string()
}

pub fn prisma_client() -> String {
r#"import { PrismaClient } from "@prisma/client";

const globalForPrisma = globalThis as unknown as { prisma: PrismaClient };

export const prisma =
  globalForPrisma.prisma ??
  new PrismaClient({ log: process.env.NODE_ENV === "development" ? ["query"] : [] });

if (process.env.NODE_ENV !== "production") globalForPrisma.prisma = prisma;
"#.to_string()
}

pub fn root_layout(app_name: &str) -> String {
    format!(r#"import type {{ Metadata }} from "next";
import "./globals.css";

export const metadata: Metadata = {{
  title: "{}",
  description: "Built with Spoke",
}};

export default function RootLayout({{ children }}: {{ children: React.ReactNode }}) {{
  return (
    <html lang="en">
      <body className="bg-gray-50 text-gray-900 antialiased">
        {{children}}
      </body>
    </html>
  );
}}
"#, app_name)
}

pub fn globals_css() -> String {
r#"@tailwind base;
@tailwind components;
@tailwind utilities;

:root { --foreground: #171717; --background: #ffffff; }
body { color: var(--foreground); background: var(--background); }
"#.to_string()
}

pub fn root_page(program: &ResolvedProgram) -> String {
    let redirect = program.auth.as_ref()
        .map(|a| a.login_redirect.clone())
        .unwrap_or("/dashboard".to_string());

    format!(r#"import {{ redirect }} from "next/navigation";
export default function Home() {{ redirect("{}"); }}
"#, redirect)
}

pub fn env_example(program: &ResolvedProgram) -> String {
    let mut env = String::new();
    env.push_str("# Database\n");
    env.push_str("DATABASE_URL=\"postgresql://user:password@localhost:5432/");
    env.push_str(&program.app_name.to_lowercase().replace(' ', "_"));
    env.push_str("\"\n\n");

    if let Some(auth) = &program.auth {
        env.push_str("# NextAuth\n");
        env.push_str("NEXTAUTH_URL=\"http://localhost:3000\"\n");
        env.push_str("NEXTAUTH_SECRET=\"your-secret-here-generate-with-openssl-rand-base64-32\"\n");

        if auth.oauth_providers.contains(&"google".to_string()) {
            env.push_str("\n# Google OAuth\n");
            env.push_str("GOOGLE_CLIENT_ID=\"\"\n");
            env.push_str("GOOGLE_CLIENT_SECRET=\"\"\n");
        }
        if auth.oauth_providers.contains(&"github".to_string()) {
            env.push_str("\n# GitHub OAuth\n");
            env.push_str("GITHUB_CLIENT_ID=\"\"\n");
            env.push_str("GITHUB_CLIENT_SECRET=\"\"\n");
        }
        if auth.magic_link {
            env.push_str("\n# Email (for magic link)\n");
            env.push_str("EMAIL_SERVER=\"smtp://user:pass@smtp.example.com:587\"\n");
            env.push_str("EMAIL_FROM=\"noreply@example.com\"\n");
        }
    }

    env
}

pub fn login_page(program: &ResolvedProgram) -> String {
    let app_name = &program.app_name;
    let signup_url = "/signup";

    format!(r#"'use client';
import {{ signIn }} from "next-auth/react";
import {{ useState }} from "react";
import {{ useRouter }} from "next/navigation";

export default function LoginPage() {{
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const router = useRouter();

  async function handleSubmit(e: React.FormEvent) {{
    e.preventDefault();
    setLoading(true);
    setError("");
    const res = await signIn("credentials", {{
      email, password, redirect: false,
    }});
    setLoading(false);
    if (res?.error) setError("Invalid email or password");
    else router.push("/dashboard");
  }}

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="w-full max-w-md bg-white rounded-2xl shadow-sm border p-8">
        <h1 className="text-2xl font-bold text-center mb-6">{}</h1>
        <form onSubmit={{handleSubmit}} className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1">Email</label>
            <input
              type="email" value={{email}} onChange={{e => setEmail(e.target.value)}}
              className="w-full border rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
              required
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Password</label>
            <input
              type="password" value={{password}} onChange={{e => setPassword(e.target.value)}}
              className="w-full border rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
              required
            />
          </div>
          {{error && <p className="text-red-500 text-sm">{{error}}</p>}}
          <button
            type="submit" disabled={{loading}}
            className="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 rounded-lg transition disabled:opacity-50"
          >
            {{loading ? "Signing in..." : "Sign in"}}
          </button>
        </form>
        <p className="text-center text-sm text-gray-500 mt-4">
          Don't have an account? <a href="{}" className="text-blue-600 hover:underline">Sign up</a>
        </p>
      </div>
    </div>
  );
}}
"#, app_name, signup_url)
}

pub fn signup_page(program: &ResolvedProgram) -> String {
    let app_name = &program.app_name;

    format!(r#"'use client';
import {{ useState }} from "react";
import {{ useRouter }} from "next/navigation";

export default function SignupPage() {{
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const router = useRouter();

  async function handleSubmit(e: React.FormEvent) {{
    e.preventDefault();
    setLoading(true);
    setError("");
    const res = await fetch("/api/auth/signup", {{
      method: "POST",
      headers: {{ "Content-Type": "application/json" }},
      body: JSON.stringify({{ name, email, password }}),
    }});
    setLoading(false);
    if (!res.ok) {{
      const data = await res.json();
      setError(data.error || "Signup failed");
    }} else {{
      router.push("/login");
    }}
  }}

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="w-full max-w-md bg-white rounded-2xl shadow-sm border p-8">
        <h1 className="text-2xl font-bold text-center mb-6">Create account — {}</h1>
        <form onSubmit={{handleSubmit}} className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1">Name</label>
            <input type="text" value={{name}} onChange={{e => setName(e.target.value)}}
              className="w-full border rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500" required />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Email</label>
            <input type="email" value={{email}} onChange={{e => setEmail(e.target.value)}}
              className="w-full border rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500" required />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Password</label>
            <input type="password" value={{password}} onChange={{e => setPassword(e.target.value)}}
              className="w-full border rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500" required />
          </div>
          {{error && <p className="text-red-500 text-sm">{{error}}</p>}}
          <button type="submit" disabled={{loading}}
            className="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 rounded-lg transition disabled:opacity-50">
            {{loading ? "Creating account..." : "Create account"}}
          </button>
        </form>
        <p className="text-center text-sm text-gray-500 mt-4">
          Already have an account? <a href="/login" className="text-blue-600 hover:underline">Sign in</a>
        </p>
      </div>
    </div>
  );
}}
"#, app_name)
}
