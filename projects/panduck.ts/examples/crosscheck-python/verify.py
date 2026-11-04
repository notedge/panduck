"""Independent Python oracle for Panduck document products.

Reads docx / pdf / markdown artifacts produced by the Node runner and checks
them with libraries Panduck does not own: python-docx, PyMuPDF and mistune.
Nothing here imports Panduck, so a Panduck defect cannot also satisfy the check.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import docx
import mistune
import pymupdf


def check_docx_table_and_list(path: Path) -> list[dict]:
    document = docx.Document(str(path))
    paragraphs = [p.text.strip() for p in document.paragraphs]
    tables = document.tables
    results = []

    results.append(
        {
            "id": "docx-heading",
            "ok": any(text == "Imported Table" for text in paragraphs),
            "detail": f"paragraphs={paragraphs}",
        }
    )
    if tables:
        rows = [[cell.text.strip() for cell in row.cells] for row in tables[0].rows]
        results.append(
            {
                "id": "docx-table-cells",
                "ok": rows == [["left", "right"], ["a", "b"]],
                "detail": f"rows={rows}",
            }
        )
    else:
        results.append({"id": "docx-table-cells", "ok": False, "detail": "no table"})
    results.append(
        {
            "id": "docx-list-items",
            "ok": "one" in paragraphs and "two" in paragraphs,
            "detail": f"paragraphs={paragraphs}",
        }
    )
    results.append(
        {
            "id": "docx-trailing-paragraph",
            "ok": any("closing paragraph" in text for text in paragraphs),
            "detail": f"paragraphs={paragraphs}",
        }
    )
    return results


def check_docx_nested(path: Path) -> list[dict]:
    document = docx.Document(str(path))
    paragraphs = [p.text.strip() for p in document.paragraphs]
    tables = document.tables
    results = [
        {
            "id": "docx-nested-items",
            "ok": all(text in paragraphs for text in ("outer", "inner", "second")),
            "detail": f"paragraphs={paragraphs}",
        },
        {
            "id": "docx-task-list",
            "ok": any("open task" in text for text in paragraphs)
            and any("done task" in text for text in paragraphs),
            "detail": f"paragraphs={paragraphs}",
        },
    ]
    if tables:
        rows = [[cell.text.strip() for cell in row.cells] for row in tables[0].rows]
        results.append(
            {
                "id": "docx-nested-table",
                "ok": rows[0] == ["left", "right"] and rows[1] == ["a", "b"],
                "detail": f"rows={rows}",
            }
        )
    else:
        results.append({"id": "docx-nested-table", "ok": False, "detail": "no table"})
    return results


def check_pdf_fonts(path: Path) -> list[dict]:
    document = pymupdf.open(str(path))
    page = document[0]
    text = page.get_text("text")
    fonts = page.get_fonts(full=True)
    embedded = [font for font in fonts if font[1] not in ("n/a", "")]
    results = [
        {
            "id": "pdf-text-extractable",
            "ok": "Latin" in text and "monospace" in text,
            "detail": f"text={text!r}",
        },
        {
            "id": "pdf-cjk-extractable",
            "ok": "中文正文" in text and "中文等宽" in text,
            "detail": f"text={text!r}",
        },
        {
            "id": "pdf-fonts-embedded",
            "ok": len(fonts) >= 3 and len(embedded) == len(fonts),
            "detail": f"fonts={[(font[3], font[1]) for font in fonts]}",
        },
    ]
    document.close()
    return results


def check_markdown_structure(path: Path) -> list[dict]:
    html = mistune.create_markdown(plugins=["table"])(path.read_text(encoding="utf-8"))
    results = [
        {"id": "markdown-table", "ok": "<table>" in html and "<th>left</th>" in html, "detail": html},
        {"id": "markdown-list", "ok": "<li>one</li>" in html and "<li>two</li>" in html, "detail": html},
        {"id": "markdown-paragraph", "ok": "closing paragraph" in html, "detail": html},
    ]
    return results


def main(argv: list[str]) -> int:
    root = Path(argv[1])
    checks = {
        "docx-table-and-list": check_docx_table_and_list(root / "table-and-list.docx"),
        "docx-nested-structures": check_docx_nested(root / "nested-structures.docx"),
        "pdf-fonts-and-cjk": check_pdf_fonts(root / "fonts-and-cjk.pdf"),
        "markdown-structure": check_markdown_structure(root / "table-and-list.md"),
    }
    summary = []
    failed = 0
    for group, results in checks.items():
        for result in results:
            summary.append({"group": group, **result})
            if not result["ok"]:
                failed += 1
    print(json.dumps({"artifacts": str(root), "checks": summary, "failed": failed}, ensure_ascii=False, indent=2))
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
