import os
import re

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

with open("assets/data/economia/potenciais.ron", "r", encoding="utf-8") as f:
    text = f.read()

for est in ["usa_iowa", "usa_ohio", "usa_american_samoa"]:
    idx = text.find(f'estado_id: "{est}"')
    if idx == -1:
        idx = text.find(f'estado_id:"{est}"')
    if idx != -1:
        end = text.find("),", idx)
        print(f"=== {est} in potenciais.ron ===")
        print(text[idx:end+2])
