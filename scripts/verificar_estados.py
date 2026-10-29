import re

with open("/home/rafael/claude/guerra-fria-jogar/assets/data/economia/estados.ron", "r", encoding="utf-8") as f:
    content = f.read()

entries = re.findall(r'\(\s*id:\s*"([^"]+)",\s*nome:\s*"([^"]+)",\s*pais_id:\s*"([^"]+)"', content)

def buscar(termo):
    print(f"\nResultados para '{termo}':")
    for sid, nome, pid in entries:
        if termo.lower() in sid.lower() or termo.lower() in nome.lower():
            print(f"  {sid} ({nome}) [{pid}]")

buscar("ruhr")
buscar("baku")
buscar("katanga")
buscar("persi")
buscar("kuwait")
buscar("saud")
buscar("irã")
buscar("iran")
buscar("iraq")
buscar("silesia")
buscar("siles")
buscar("saxo")
buscar("baviera")
buscar("kuznetsk")
buscar("kuzbass")
buscar("kemerovo")
