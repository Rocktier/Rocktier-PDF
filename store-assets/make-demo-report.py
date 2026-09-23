"""生成 Rocktier PDF 商店截图用的合成示例文档。

家族硬规矩：公开截图里不得出现任何真实客户/个人内容（见 CAD 的 store-listing §5）。
所以这里**全部内容都是编造的** —— 虚构建筑名、虚构设备编号、虚构日期。

产出 4 页，覆盖截图需要的画面：
  1  标题 + 摘要段落（含可搜索词 "inspection"，重复出现便于演示查找）
  2  设备清单表格（演示表格渲染与缩放清晰度）
  3  行动项编号清单 + 现场照片占位框
  4  AcroForm 文本框 + 签署区（演示表单填写功能）
"""
from reportlab.lib.pagesizes import A4
from reportlab.lib.colors import HexColor
from reportlab.lib.units import mm
from reportlab.pdfgen import canvas
import pathlib

OUT = pathlib.Path(__file__).resolve().parent   # 与脚本同目录，任何机器上都能跑
OUT.mkdir(parents=True, exist_ok=True)
PDF = OUT / "demo-report.pdf"

W, H = A4
M = 22 * mm                       # 页边距
INK = HexColor("#111111")
MUTED = HexColor("#707070")
RULE = HexColor("#dcdcdc")
BAND = HexColor("#f2f2f2")
ACCENT = HexColor("#b4451f")       # 与家族品牌点同色系，但不抢眼

c = canvas.Canvas(str(PDF), pagesize=A4)
c.setTitle("Facilities Inspection Report")
c.setAuthor("Rocktier")
c.setSubject("Synthetic sample document")


def footer(page_no):
    c.setStrokeColor(RULE)
    c.setLineWidth(0.6)
    c.line(M, 16 * mm, W - M, 16 * mm)
    c.setFont("Helvetica", 8)
    c.setFillColor(MUTED)
    c.drawString(M, 11 * mm, "Riverside Hall — Facilities Inspection Report")
    c.drawRightString(W - M, 11 * mm, f"Page {page_no} of 4")


def heading(text, y, size=13):
    c.setFont("Helvetica-Bold", size)
    c.setFillColor(INK)
    c.drawString(M, y, text)
    return y - 8


def para(text, x, y, width, size=10, leading=14.5, color=INK, font="Helvetica"):
    """极简折行：按字符估算宽度，够用即可。"""
    c.setFont(font, size)
    c.setFillColor(color)
    words, line = text.split(), ""
    for w in words:
        probe = (line + " " + w).strip()
        if c.stringWidth(probe, font, size) > width and line:
            c.drawString(x, y, line)
            y -= leading
            line = w
        else:
            line = probe
    if line:
        c.drawString(x, y, line)
        y -= leading
    return y


# ─────────────────────────── 第 1 页 ───────────────────────────
c.setFillColor(BAND)
c.rect(0, H - 46 * mm, W, 46 * mm, stroke=0, fill=1)
c.setFillColor(ACCENT)
c.rect(0, H - 46 * mm, 5 * mm, 46 * mm, stroke=0, fill=1)

c.setFillColor(INK)
c.setFont("Helvetica-Bold", 20)
c.drawString(M, H - 24 * mm, "Facilities Inspection Report")
c.setFont("Helvetica", 10.5)
c.setFillColor(MUTED)
c.drawString(M, H - 32 * mm, "Riverside Hall · Block C · Quarterly cycle")

y = H - 62 * mm
y = heading("1. Summary", y)
y -= 4
y = para(
    "This inspection covers the mechanical plant, the two passenger lifts and the "
    "roof-level plant room at Riverside Hall, Block C. It was carried out over a "
    "single morning, with the building in normal operation.",
    M, y, W - 2 * M,
)
y -= 6
y = para(
    "Seven items were raised. Four are routine and can be handled by the on-site "
    "team; three need a specialist visit. No item is classified as urgent, and the "
    "building remains fit for use. The next inspection is due in the following "
    "quarter, and the inspection record should be read alongside the maintenance log.",
    M, y, W - 2 * M,
)
y -= 10

c.setFillColor(BAND)
c.rect(M, y - 22 * mm, W - 2 * M, 22 * mm, stroke=0, fill=1)
c.setFillColor(INK)
c.setFont("Helvetica-Bold", 9.5)
c.drawString(M + 5 * mm, y - 7 * mm, "Outcome")
c.setFont("Helvetica", 9)
c.setFillColor(MUTED)
c.drawString(M + 5 * mm, y - 13 * mm, "Pass, with actions. 7 items raised · 0 urgent · 3 requiring a specialist visit.")
c.drawString(M + 5 * mm, y - 18.5 * mm, "Inspection reference RH-C-2609 · Recorded by the site maintenance team.")
footer(1)
c.showPage()

# ─────────────────────────── 第 2 页 ───────────────────────────
y = H - 28 * mm
y = heading("2. Equipment inventory", y)
y -= 4
y = para("Plant recorded during the inspection, with the condition noted on the day.", M, y, W - 2 * M, size=9, color=MUTED)
y -= 8

cols = [M, M + 62 * mm, M + 108 * mm, W - M]
rows = [
    ("Asset", "Location", "Condition"),
    ("AHU-03", "Roof plant room", "Filters due"),
    ("AHU-04", "Roof plant room", "Serviceable"),
    ("LIFT-01", "Core, north", "Serviceable"),
    ("LIFT-02", "Core, south", "Door sensor"),
    ("PMP-11", "Basement", "Serviceable"),
    ("PMP-12", "Basement", "Seal weeping"),
    ("TANK-02", "Roof plant room", "Serviceable"),
]
row_h = 9.5 * mm
c.setFont("Helvetica-Bold", 9)
c.setFillColor(INK)
for i, txt in enumerate(rows[0]):
    c.drawString(cols[i] + (2 * mm if i else 0), y, txt)
y -= 3
c.setStrokeColor(INK)
c.setLineWidth(1)
c.line(M, y, W - M, y)
y -= row_h
c.setFont("Helvetica", 9)
for r in rows[1:]:
    c.setFillColor(INK if r[2] == "Serviceable" else ACCENT)
    for i, txt in enumerate(r):
        c.drawString(cols[i] + (2 * mm if i else 0), y + 3 * mm, txt)
    c.setStrokeColor(RULE)
    c.setLineWidth(0.5)
    c.line(M, y, W - M, y)
    y -= row_h

footer(2)
c.showPage()

# ─────────────────────────── 第 3 页 ───────────────────────────
y = H - 28 * mm
y = heading("3. Actions", y)
y -= 6
actions = [
    "Replace the supply filters on AHU-03 before the next inspection.",
    "Adjust the door sensor on LIFT-02 and re-test the closing cycle.",
    "Reseal the pump gland on PMP-12 and confirm there is no weeping.",
    "Clear the stored items blocking the plant room access route.",
    "Record the filter part numbers in the maintenance log.",
]
for i, a in enumerate(actions, 1):
    c.setFillColor(ACCENT)
    c.setFont("Helvetica-Bold", 10)
    c.drawString(M, y, f"{i}.")
    c.setFillColor(INK)
    c.setFont("Helvetica", 10)
    c.drawString(M + 7 * mm, y, a)
    y -= 8 * mm

y -= 6
c.setStrokeColor(RULE)
c.setLineWidth(0.8)
c.rect(M, y - 52 * mm, W - 2 * M, 52 * mm, stroke=1, fill=0)
c.setFillColor(MUTED)
c.setFont("Helvetica", 9)
c.drawCentredString(W / 2, y - 26 * mm, "Photograph of the roof plant room recorded during the inspection")
footer(3)
c.showPage()

# ─────────────────────────── 第 4 页 ───────────────────────────
y = H - 28 * mm
y = heading("4. Sign-off", y)
y -= 6
y = para(
    "Complete the field below and sign to confirm the actions have been reviewed. "
    "This page is the inspection record for the quarter.",
    M, y, W - 2 * M,
)
y -= 12

c.setFillColor(INK)
c.setFont("Helvetica-Bold", 9)
c.drawString(M, y, "Reviewer notes")
y -= 3
field_h = 18 * mm
c.acroForm.textfield(
    name="reviewer_notes",
    tooltip="Reviewer notes",
    x=M,
    y=y - field_h,
    width=W - 2 * M,
    height=field_h,
    borderStyle="inset",
    forceBorder=True,
    fontName="Helvetica",
    fontSize=10,
)
y -= field_h + 22

c.setFillColor(INK)
c.setFont("Helvetica-Bold", 9)
c.drawString(M, y, "Reviewed by")
c.drawString(M + 90 * mm, y, "Date")
c.setStrokeColor(MUTED)
c.setLineWidth(0.8)
c.line(M, y - 12 * mm, M + 80 * mm, y - 12 * mm)
c.line(M + 90 * mm, y - 12 * mm, W - M, y - 12 * mm)
footer(4)
c.save()

print(f"  ✅ 已生成 {PDF}  ({PDF.stat().st_size} B)")
