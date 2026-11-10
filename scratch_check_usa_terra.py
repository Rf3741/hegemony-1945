import os
import re

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

with open("assets/data/economia/potenciais.ron", "r", encoding="utf-8") as f:
    text = f.read()

total_pot_terra_usa = 0
for m in re.finditer(r'estado_id:\s*"usa_([^"]+)",\s*terra_aravel:\s*(\d+)', text):
    total_pot_terra_usa += int(m.group(2))

print(f"Total terra_aravel in potenciais.ron for USA: {total_pot_terra_usa}")
