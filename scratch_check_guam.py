import os
import subprocess
import re

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
for line in proc.stdout.split("\n"):
    if 'id:"gum_00"' in line or 'nome:"Guam"' in line:
        print("Guam in jogavel:")
        for m in re.finditer(r'tipo_predio_id:"([^"]+)",nivel:(\d+)', line):
            print(" ", m.group(1), m.group(2))
