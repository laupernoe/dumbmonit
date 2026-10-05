import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig } from 'vite';

const PROXY = { target: 'http://localhost:8080', changeOrigin: true };

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit(),
		// i18n : compile messages/{locale}.json en fonctions ES tree-shakées
		// (src/lib/paraglide, généré — jamais commité). Pas de routage par URL :
		// la langue est détectée côté client (localStorage, puis la langue du
		// navigateur, puis l'anglais) puisque l'app est une SPA statique (ssr=false).
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide',
			strategy: ['localStorage', 'preferredLanguage', 'baseLocale'],
			emitTsDeclarations: true
		})
	],
	// En développement, le front tourne sur Vite et le backend Rust sur 8080.
	// Ce proxy évite toute configuration CORS côté serveur.
	// `preview` en bénéficie aussi, pour éprouver le build face au vrai backend.
	server: {
		proxy: { '/api': PROXY },
		// La documentation Markdown (`docs/` à la racine du dépôt) vit hors de `web/` :
		// Vite doit être autorisé à la servir en développement. Le build, lui,
		// l'embarque sans restriction. Les chemins sont relatifs à `web/`.
		fs: { allow: ['.', '../docs'] }
	},
	preview: { proxy: { '/api': PROXY } }
});
