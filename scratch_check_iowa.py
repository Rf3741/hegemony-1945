import os
import subprocess
import re

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

# Check estados.ron for usa_iowa and usa_ohio
with open("assets/data/economia/estados.ron", "r", encoding="utf-8") as f:
    est_text = f.read()

for est_id in ["usa_iowa", "usa_ohio", "usa_american_samoa"]:
    idx = est_text.find(f'id: "{est_id}"')
    if idx == -1:
        idx = est_text.find(f'id:"{est_id}"')
    if idx != -1:
        end = est_text.find("),", idx)
        print(f"=== {est_id} in estados.ron ===")
        print(est_text[idx:end+2])

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
for line in proc.stdout.split("\n"):
    if 'id:"usa_3529"' in line or 'id: "usa_3529"' in line:
        print("Iowa in jogavel:")
        for m in re.finditer(r'tipo_predio_id:"([^"]+)",nivel:(\d+)', line):
            print(" ", m.group(1), m.group(2))

