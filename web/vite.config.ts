import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig } from 'vite';

// Node's `process`, read through globalThis: the web project has no Node type definitions.
const env = (globalThis as { process?: { env: Record<string, string | undefined> } }).process?.env;
// Démo statique : l'application vit sous un sous-chemin (`/demo`). Lu une seule fois ici et
// transmis au code (`VITE_BASE_PATH`, voir `src/lib/paths.ts`) ; vide pour le build normal.
const BASE_PATH = (env?.DUMBMONIT_BASE_PATH ?? '').replace(/\/+$/, '');
if (BASE_PATH && !BASE_PATH.startsWith('/')) throw new Error('DUMBMONIT_BASE_PATH must start with "/"');
const PROXY = { target: env?.DUMBMONIT_DEV_API ?? 'http://localhost:8080', changeOrigin: true };

export default defineConfig({
	// Lu par `import.meta.env.VITE_BASE_PATH` ; la valeur vide laisse le build normal inchangé.
	define: { 'import.meta.env.VITE_BASE_PATH': JSON.stringify(BASE_PATH) },
	plugins: [
		tailwindcss(),
		sveltekit({
			preprocess: vitePreprocess(),
			// Le binaire Rust embarque un site statique : pas de serveur Node en production.
			// `fallback` active le mode SPA, indispensable pour les routes dynamiques
			// comme /targets/42 qui sont résolues côté client.
			adapter: adapter({
				pages: 'build',
				assets: 'build',
				fallback: 'index.html',
				precompress: false,
				strict: false
			}),
			// Sous-chemin de la démo statique ; rien du tout pour le build normal. Chemins
			// absolus : la page de repli sert aussi des routes profondes (/demo/targets/3).
			...(BASE_PATH ? { paths: { base: BASE_PATH, relative: false } } : {}),
			// Aucune page n'est pré-rendue par défaut : tout dépend de l'API au runtime.
			prerender: { entries: [] },
			// Identifiant de version de l'application (`_app/version.json`, recopié dans
			// le JavaScript). Par défaut SvelteKit prend l'heure de la construction, et
			// deux constructions du même commit ne sont plus identiques : le Dockerfile
			// passe le commit (`DUMBMONIT_BUILD`) ou sa date (`SOURCE_DATE_EPOCH`).
			version: {
				name: env?.DUMBMONIT_BUILD || env?.SOURCE_DATE_EPOCH || Date.now().toString()
			}
		}),
		// i18n : compile messages/{locale}.json en fonctions ES tree-shakées
		// (src/lib/paraglide, généré — jamais commité). Pas de routage par URL :
		// la langue est détectée côté client (localStorage, puis la langue du
		// navigateur, puis l'anglais) puisque l'app est une SPA statique (ssr=false).
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide',
			strategy: ['localStorage', 'preferredLanguage', 'baseLocale'],
			emitTsDeclarations: true,
			// Un module par langue (`messages/fr.js`…) : l'application n'en charge
			// qu'une (voir l'alias ci-dessous), au lieu des neuf dans un seul bloc.
			outputStructure: 'locale-modules'
		})
	],
	resolve: {
		alias: [
			// `m` vient de `src/lib/i18n/messages.ts` : mêmes appels, mais seule la langue
			// active est téléchargée. Les types viennent toujours du code généré.
			{ find: /^#lib\/paraglide\/messages\.js$/, replacement: '/src/lib/i18n/messages.ts' }
		]
	},
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
