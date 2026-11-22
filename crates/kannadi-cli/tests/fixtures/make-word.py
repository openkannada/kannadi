"""Writes legacy.docx from lines of the glyph fixtures, the text kannadi should read
from it, and the .doc and .rtf copies LibreOffice saves from it.

Usage: python3 make-word.py [path to soffice]
"""

import pathlib
import subprocess
import sys
import tempfile
import zipfile
from xml.sax.saxutils import escape

HERE = pathlib.Path(__file__).resolve().parent
LIB_FIXTURES = HERE.parents[2] / "kannadi" / "tests" / "fixtures"


def lines(name):
    """(glyph text, expected Unicode) for each data line of a fixture TSV."""
    out = []
    for line in (LIB_FIXTURES / name).read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        codes, expected = line.split("\t")[:2]
        glyphs = bytes(int(h, 16) for h in codes.split()).decode("cp1252")
        out.append((glyphs, expected))
    return out


NUDI = lines("nudi.tsv")
AKRUTI = lines("akruti.tsv")
W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
R = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'


def run(text, font=None):
    props = f'<w:rPr><w:rFonts w:ascii="{font}" w:hAnsi="{font}" w:cs="{font}"/></w:rPr>' if font else ""
    return f'<w:r>{props}<w:t xml:space="preserve">{escape(text)}</w:t></w:r>'


def paragraph(*runs, style=None):
    props = f'<w:pPr><w:pStyle w:val="{style}"/></w:pPr>' if style else ""
    return f"<w:p>{props}{''.join(runs)}</w:p>"


def cell(p):
    return f'<w:tc><w:tcPr><w:tcW w:w="4000" w:type="dxa"/></w:tcPr>{p}</w:tc>'


BODY = [
    (paragraph(run(NUDI[0][0], "Nudi 01 e")), NUDI[0][1]),
    (paragraph(run(NUDI[1][0], "BRH Kannada")), NUDI[1][1]),
    (
        paragraph(run("Rural Development Department, 2002: ", "Times New Roman"), run(NUDI[2][0], "Nudi 01 e")),
        "Rural Development Department, 2002: " + NUDI[2][1],
    ),
    (paragraph(run("ಕರ್ನಾಟಕ ಸರ್ಕಾರ", "Noto Sans Kannada")), "ಕರ್ನಾಟಕ ಸರ್ಕಾರ"),
    (paragraph(run("2002", "Nudi 04 k")), "೨೦೦೨"),
    (paragraph(run(NUDI[4][0]), style="NudiBody"), NUDI[4][1]),
    (paragraph(run(AKRUTI[0][0], "AkrutiKndPadma")), AKRUTI[0][1]),
]
TABLE = (
    '<w:tbl><w:tblPr><w:tblW w:w="8000" w:type="dxa"/></w:tblPr>'
    '<w:tblGrid><w:gridCol w:w="4000"/><w:gridCol w:w="4000"/></w:tblGrid><w:tr>'
    + cell(paragraph(run(NUDI[5][0], "Nudi 01 e")))
    + cell(paragraph(run("Total", "Times New Roman")))
    + "</w:tr></w:tbl>"
)
HEADER = (NUDI[6][0], NUDI[6][1])

STYLES = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles {W}>
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman"/></w:rPr></w:rPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="NudiBody"><w:name w:val="Nudi Body"/><w:basedOn w:val="Normal"/>
<w:rPr><w:rFonts w:ascii="Nudi 01 e" w:hAnsi="Nudi 01 e" w:cs="Nudi 01 e"/></w:rPr></w:style>
</w:styles>"""


def docx(path):
    body = "".join(p for p, _ in BODY) + TABLE + paragraph(run("End", "Times New Roman"))
    sect = '<w:sectPr><w:headerReference w:type="default" r:id="rIdHeader"/></w:sectPr>'
    parts = {
        "[Content_Types].xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
<Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>
</Types>""",
        "_rels/.rels": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>""",
        "word/_rels/document.xml.rels": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
<Relationship Id="rIdHeader" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>
</Relationships>""",
        "word/document.xml": f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
        f"<w:document {W} {R}><w:body>{body}{sect}</w:body></w:document>",
        "word/styles.xml": STYLES,
        "word/header1.xml": f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
        f"<w:hdr {W} {R}>{paragraph(run(HEADER[0], 'Nudi 01 e'))}</w:hdr>",
    }
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in parts.items():
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data.encode("utf-8"))


def main():
    soffice = sys.argv[1] if len(sys.argv) > 1 else "soffice"
    docx(HERE / "legacy.docx")
    body = [text for _, text in BODY] + [NUDI[5][1], "Total", "End"]
    (HERE / "legacy.expected.txt").write_text("\n".join(body) + "\n\n" + HEADER[1] + "\n", encoding="utf-8")
    with tempfile.TemporaryDirectory() as profile:
        for target in ["doc:MS Word 97", "rtf"]:
            subprocess.run(
                [
                    soffice,
                    f"-env:UserInstallation=file://{profile}",
                    "--headless",
                    "--convert-to",
                    target,
                    "--outdir",
                    str(HERE),
                    str(HERE / "legacy.docx"),
                ],
                check=True,
            )


if __name__ == "__main__":
    main()
