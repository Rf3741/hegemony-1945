import proto_sim3

p = next(p for p in proto_sim3.provs if p['nome'] == 'Ilha Baker')
print("Baker Island predios:")
for t, lvl in proto_sim3.prov_levels[p['id']].items():
    if lvl > 0:
        print(f"  {t:30}: {lvl}")
