#!/usr/bin/env python3
"""Converte ne_10m_rivers_lake_centerlines (shapefile, dominio publico) em assets/geo/rios.ron
na projecao Miller do mapa (coordenadas de mundo Bevy). Uso: gerar_rios_ron.py <prefixo_shp> <saida.ron>"""
import math, struct, sys
W, H = 5632, 2048
YMAX = 2.3034125433763912
def proj(lon, lat):
    y = 1.25 * math.log(math.tan(math.pi/4 + 0.4*math.radians(lat)))
    return ((lon+180)/360*W - W/2, H/2 - (YMAX-y)/(2*YMAX)*H)
def dbf(p):
    f = open(p+'.dbf','rb').read()
    n, hl, rl = struct.unpack('<xxxxIHH', f[:12]); fl=[]; o=32
    while f[o] != 0x0d: fl.append((f[o:o+11].split(b'\0')[0].decode(), f[o+16])); o += 32
    out=[]
    for i in range(n):
        r=f[hl+i*rl:hl+(i+1)*rl]; q=1; d={}
        for nm,ln in fl:
            if nm in ('scalerank','featurecla','name'): d[nm]=r[q:q+ln].split(b'\0')[0].decode('latin1').strip()
            q+=ln
        out.append(d)
    return out
def shp(p):
    s=open(p+'.shp','rb').read(); o=100; out=[]
    while o < len(s):
        _, cl = struct.unpack('>ii', s[o:o+8]); o += 8
        c=s[o:o+cl*2]; o += cl*2
        if struct.unpack('<i', c[:4])[0] != 3: out.append([]); continue
        npart, npts = struct.unpack('<ii', c[36:44])
        parts=list(struct.unpack('<%di'%npart, c[44:44+4*npart]))+[npts]
        v=struct.unpack('<%dd'%(2*npts), c[44+4*npart:44+4*npart+16*npts])
        pts=[(v[2*i],v[2*i+1]) for i in range(npts)]
        out.append([pts[parts[k]:parts[k+1]] for k in range(npart)])
    return out
def simplificar(pts, tol):
    if len(pts) < 3: return pts
    (x1,y1),(x2,y2)=pts[0],pts[-1]; dx,dy=x2-x1,y2-y1; n=math.hypot(dx,dy) or 1e-9
    m,i=0,0
    for k in range(1,len(pts)-1):
        d=abs(dy*(pts[k][0]-x1)-dx*(pts[k][1]-y1))/n
        if d>m: m,i=d,k
    if m>tol: return simplificar(pts[:i+1],tol)[:-1]+simplificar(pts[i:],tol)
    return [pts[0],pts[-1]]
d, g = dbf(sys.argv[1]), shp(sys.argv[1])
linhas=[]
for r, partes in zip(d, g):
    if r.get('featurecla') not in ('River','Lake Centerline'): continue
    rank=int(float(r.get('scalerank') or 9))
    for p in partes:
        q=[proj(lon,lat) for lon,lat in p]
        q=simplificar(q,0.35)
        if len(q)>=2: linhas.append((min(rank,12), q))
with open(sys.argv[2],'w') as f:
    f.write('// Natural Earth ne_10m_rivers_lake_centerlines (dominio publico), projecao Miller do mapa (mundo Bevy).\n')
    f.write('// ordem: scalerank NE (1 = maior rio). Gerado por scripts/gerar_rios_ron.py\n(\n    rios: [\n')
    for rank,q in linhas:
        f.write('        (ordem: %d, pontos: [%s]),\n' % (rank, ','.join('(%.1f,%.1f)'%p for p in q)))
    f.write('    ],\n)\n')
print(len(linhas), 'linhas')
