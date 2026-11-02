import { defineConfig } from '@vmz/plugin';

/** Static CDN — publish only `dist/cdn` (profile `name`), not the whole `dist/`. */
export default defineConfig({
    delivery: {
        default: 'static',
        profiles: {
            static: {
                host: 'browser',
                assembly: 'web-static',
                name: 'cdn',
            },
        },
    },
});
