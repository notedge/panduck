/**

 * @vmz/vmz npm tarballs omit host-runtime-files.json and delivery-runtime-files.json.

 * Materialize them before `vmz check` / `vmz build` until upstream fixes publish.

 */



import { copyFileSync, existsSync, readFileSync } from 'node:fs';

import { createRequire } from 'node:module';

import path from 'node:path';

import { fileURLToPath } from 'node:url';



const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');



const MANIFESTS = [

    { name: 'host-runtime-files.json', bundled: 'vmz-host-runtime-files.json' },

    { name: 'delivery-runtime-files.json', bundled: 'vmz-delivery-runtime-files.json' },

];



function resolveVmzPackageRoot() {

    const searchRoots = [process.cwd(), root];

    for (const base of searchRoots) {

        const require = createRequire(path.join(base, 'package.json'));

        try {

            const entry = require.resolve('@vmz/vmz');

            let dir = path.dirname(entry);

            for (let i = 0; i < 4; i += 1) {

                const pkgJson = path.join(dir, 'package.json');

                if (existsSync(pkgJson)) {

                    const pkg = JSON.parse(readFileSync(pkgJson, 'utf8'));

                    if (pkg.name === '@vmz/vmz') {

                        return dir;

                    }

                }

                dir = path.dirname(dir);

            }

        } catch {

            /* try next root */

        }

    }

    return null;

}



function ensureManifest(vmzRoot, { name, bundled }) {

    const manifestPath = path.join(vmzRoot, name);

    if (existsSync(manifestPath)) {

        return false;

    }

    const bundledPath = path.join(root, 'scripts', bundled);

    if (existsSync(bundledPath)) {

        copyFileSync(bundledPath, manifestPath);

    } else {

        throw new Error(`ensure-vmz-host-manifest: missing bundled ${bundled}`);

    }

    console.log(`ensure-vmz-host-manifest: wrote ${manifestPath}`);

    return true;

}



const vmzRoot = resolveVmzPackageRoot();

if (!vmzRoot) {

    console.warn('ensure-vmz-host-manifest: @vmz/vmz not installed — skip');

    process.exit(0);

}



for (const spec of MANIFESTS) {

    ensureManifest(vmzRoot, spec);

}


