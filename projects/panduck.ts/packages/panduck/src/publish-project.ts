/**
 * Markdown project materialization on the TS side.
 *
 * Mirrors the staging-directory semantics of `document-convert`'s publish layer
 * without exposing a library function that reads or writes paths. The staging
 * directory is a sibling of the destination, the whole layout is written there,
 * and only then is it renamed into place.
 */

import { mkdir, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

export type PublishableProject = {
    indexMarkdown: string;
    chapters: Array<{ path: string; markdown: string }>;
    assets: Array<{ path: string; bytes: Uint8Array }>;
};

export type PublishedProject = {
    outputDir: string;
    indexPath: string;
    chapterPaths: string[];
    assetPaths: string[];
    reportPath: string;
};

/** Materializes `index.md`, chapters, assets and the report into `outputDir`. */
export async function publishMarkdownProject(
    outputDir: string,
    project: PublishableProject,
    reportJson: string,
): Promise<PublishedProject> {
    const stagingDir = stagingDirPath(outputDir);
    await rm(stagingDir, { recursive: true, force: true });
    await mkdir(stagingDir, { recursive: true });

    try {
        await writeFile(join(stagingDir, "index.md"), project.indexMarkdown, "utf8");
        for (const chapter of project.chapters) {
            const chapterPath = join(stagingDir, chapter.path);
            await mkdir(dirname(chapterPath), { recursive: true });
            await writeFile(chapterPath, chapter.markdown, "utf8");
        }
        for (const asset of project.assets) {
            if (!asset.path.startsWith("assets/")) {
                throw new Error(`asset path must start with assets/: ${asset.path}`);
            }
            const assetPath = join(stagingDir, asset.path);
            await mkdir(dirname(assetPath), { recursive: true });
            await writeFile(assetPath, asset.bytes);
        }
        await writeFile(join(stagingDir, "panduck.report.json"), reportJson, "utf8");
        await replaceDirectory(stagingDir, outputDir);
    } finally {
        await rm(stagingDir, { recursive: true, force: true });
    }

    return {
        outputDir,
        indexPath: join(outputDir, "index.md"),
        chapterPaths: project.chapters.map((chapter) => chapter.path),
        assetPaths: project.assets.map((asset) => asset.path),
        reportPath: join(outputDir, "panduck.report.json"),
    };
}

function stagingDirPath(outputDir: string): string {
    const name = outputDir.split(/[\\/]/).filter(Boolean).pop() ?? "output";
    const nanos = process.hrtime.bigint().toString();
    return join(dirname(outputDir), `.${name}.${process.pid}.${nanos}.staging`);
}

async function replaceDirectory(stagingDir: string, outputDir: string): Promise<void> {
    try {
        await rename(stagingDir, outputDir);
    } catch {
        await rm(outputDir, { recursive: true, force: true });
        await rename(stagingDir, outputDir);
    }
}
