import os
import re

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

with open("assets/data/economia/potenciais.ron", "r", encoding="utf-8") as f:
    text = f.read()

for est in ["usa_minnesota", "usa_texas", "usa_iowa", "usa_nova_iorque", "usa_ohio", "usa_guam"]:
    idx = text.find(f'estado_id: "{est}"')
    if idx != -1:
        end = text.find("),", idx)
        print(f"=== {est} ===")
        print(text[idx:end+2])
