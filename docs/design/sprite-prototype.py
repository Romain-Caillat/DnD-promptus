import json, colorsys
W, H = 20, 26
BASE = {
 'skin':'#F0C09A','dwarfskin':'#E8A982','hair_or':'#D9762B','hair_br':'#7A4A2A','metal':'#C9CFD9','horn':'#EDE3C8',
 'armor':'#4F6D9A','leather':'#7A5233','gold':'#F2C14E','pants':'#5A4636','boot':'#3B2A20','wood':'#8A5A33',
 'eyew':'#FFFFFF','eye':'#1B1426','green':'#3F7D4E','green_dk':'#2C5A3A','tunic':'#6E8F3A','purple':'#4B3A6B','purple_dk':'#33284A',
 'grey':'#8A8F99','gob':'#7DB04C','gob_eye':'#F2D33A','red':'#B8352B','bone':'#EDE3C8','string':'#E6E0D0',
 'jacket':'#6B6A3A','jeans':'#3E5470','pack':'#7A5233','cap':'#A3362E','zskin':'#8FA37A','zshirt':'#6B6F78','zpants':'#4A4A55','zeye':'#F2EBA0',
 'plate':'#C9CFD9','visor':'#6FD3FF','gun':'#3A3F48','chitin':'#3E7F7A','chitin_dk':'#2A5552','acid':'#C8FF4A'
}
NOSHADE={'eyew','eye','gob_eye','string','gold','zeye','visor','acid'}
OUT='#1B1426'
def adj(hexc, f):
    r,g,b=[int(hexc[i:i+2],16)/255 for i in (1,3,5)]
    h,l,s=colorsys.rgb_to_hls(r,g,b)
    l=max(0,min(1,l*f if f<1 else l+(1-l)*(f-1)))
    r,g,b=colorsys.hls_to_rgb(h,l,s)
    return '#%02X%02X%02X'%(round(r*255),round(g*255),round(b*255))
class C:
    def __init__(s): s.g=[[None]*W for _ in range(H)]
    def px(s,x,y,c):
        x=int(round(x)); y=int(round(y))
        if 0<=x<W and 0<=y<H: s.g[y][x]=c
    def rect(s,x0,y0,x1,y1,c):
        for y in range(y0,y1+1):
            for x in range(x0,x1+1): s.px(x,y,c)
    def ell(s,cx,cy,rx,ry,c,ymin=-99,ymax=99):
        for y in range(H):
            for x in range(W):
                if ((x-cx)/rx)**2+((y-cy)/ry)**2<=1 and ymin<=y<=ymax: s.g[y][x]=c
    def line(s,x0,y0,x1,y1,c):
        n=max(abs(x1-x0),abs(y1-y0)) or 1
        for i in range(n+1): s.px(x0+(x1-x0)*i/n,y0+(y1-y0)*i/n,c)
    def render(s, mirror=False):
        g=s.g
        out=[[None]*W for _ in range(H)]
        filled=lambda x,y: 0<=x<W and 0<=y<H and g[y][x] is not None
        for y in range(H):
            for x in range(W):
                c=g[y][x]
                if c is None:
                    if any(filled(x+dx,y+dy) for dx,dy in ((1,0),(-1,0),(0,1),(0,-1))): out[y][x]=OUT
                    continue
                base=BASE[c]
                if c in NOSHADE: out[y][x]=base; continue
                top=not filled(x,y-1) or g[y-1][x]!=c
                back=not filled(x-1,y)
                bot=not filled(x,y+1)
                if top and not bot: out[y][x]=adj(base,1.28)
                elif back or bot: out[y][x]=adj(base,0.72)
                else: out[y][x]=base
        if mirror: out=[r[::-1] for r in out]
        return out
def encode(frames):
    pal={}; letters='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'
    res=[]
    for f in frames:
        rows=[]
        for r in f:
            s=''
            for c in r:
                if c is None: s+='.'
                else:
                    if c not in pal: pal[c]=letters[len(pal)]
                    s+=pal[c]
            rows.append(s)
        res.append(rows)
    return {v:k for k,v in pal.items()}, res

def legs(c, pose, xb, xf, top, col='pants', boot='boot', w=2):
    # pose 0: standing, 1: stride
    if pose==0:
        c.rect(xb,top,xb+w-1,23,col); c.rect(xf,top,xf+w-1,23,col)
        c.rect(xb-1,24,xb+w-1,25,boot); c.rect(xf,24,xf+w,25,boot)
    else:
        c.line(xb,top,xb-3,22,col); c.line(xb+w-1,top,xb+w-4,22,col); c.rect(xb-4,22,xb-4+w-1,22,col)
        c.rect(xb-5,23,xb-3+w-1,24,boot)
        c.line(xf,top,xf+3,23,col); c.line(xf+w-1,top,xf+w+2,23,col)
        c.rect(xf+3,24,xf+w+3,25,boot)

def borin(pose=0, dy=0):
    c=C()
    c.rect(3,14+dy,4,19+dy,'armor')                 # back arm
    legs(c,pose,6,10,21,w=3)
    c.rect(4,13+dy,13,21,'armor')                    # body
    c.rect(4,19,13,20,'leather'); c.px(11,19,'gold'); c.px(11,20,'gold')
    c.ell(9,9+dy,4.6,4.6,'dwarfskin')                # head
    c.ell(9,6+dy,5.4,3.6,'metal',ymax=6+dy)          # helmet dome
    c.rect(3,6+dy,14,7+dy,'metal')                   # rim
    for (x,y) in [(3,4),(2,3),(2,2),(1,1)]: c.px(x,y+dy,'horn')
    for (x,y) in [(14,4),(15,3),(15,2),(16,1)]: c.px(x,y+dy,'horn')
    c.ell(10,13+dy,5.2,4.2,'hair_or',ymin=10+dy)      # beard
    c.rect(11,10+dy,14,10+dy,'hair_or')              # moustache
    c.px(11,9+dy,'eyew'); c.px(12,9+dy,'eye')
    c.px(14,9+dy,'dwarfskin'); c.px(15,10+dy,'dwarfskin')  # nose
    c.line(16,6+dy,16,22+dy,'wood')                  # axe handle
    c.rect(17,6+dy,19,11+dy,'metal'); c.px(17,5+dy,'metal'); c.px(17,12+dy,'metal')
    c.rect(12,14+dy,14,17+dy,'armor')                # front arm
    c.rect(15,16+dy,16,17+dy,'dwarfskin')            # hand on handle
    return c

def lyra(pose=0, dy=0):
    c=C()
    # bow on back
    for y in range(3,21): 
        x=3-int(round(2.2*(1-((y-12)/9)**2)))
        c.px(x+1,y+dy,'wood')
    c.line(4,3+dy,4,20+dy,'string')
    c.rect(4,9+dy,6,20,'green_dk')                    # cape
    legs(c,pose,7,10,19,col='pants',w=2)
    c.rect(6,12+dy,12,19,'tunic')
    c.rect(6,17,12,17,'leather'); c.px(10,17,'gold')
    c.ell(9.5,7+dy,4.6,4.6,'green')                  # hood
    c.ell(11,8.5+dy,2.8,3,'skin')                    # face
    c.px(9,6+dy,'hair_br'); c.px(10,5+dy,'hair_br'); c.px(9,7+dy,'hair_br')
    c.px(12,8+dy,'eye'); c.px(14,9+dy,'skin')
    c.rect(11,13+dy,12,16+dy,'tunic'); c.px(13,16+dy,'skin'); c.px(13,17+dy,'skin')
    return c

def sef(pose=0, dy=0):
    c=C()
    c.rect(4,10+dy,6,19,'purple_dk')                 # cape
    legs(c,pose,7,10,19,col='purple_dk',w=2)
    c.rect(6,12+dy,12,19,'grey')
    c.rect(6,16,12,16,'leather'); c.px(8,16,'gold')
    c.ell(9.5,7+dy,4.6,4.6,'purple')                 # hood
    c.ell(11,8+dy,2.6,2.6,'skin')
    c.rect(9,9+dy,14,10+dy,'purple_dk')              # mask
    c.px(12,7+dy,'eyew'); c.px(13,7+dy,'eye')
    c.rect(11,13+dy,13,16+dy,'purple'); c.px(13,17+dy,'skin'); c.px(14,16+dy,'skin')
    c.line(14,15+dy,17,12+dy,'metal'); c.px(13,17+dy,'leather')
    return c

def gobelin(pose=0, dy=0):
    c=C()
    c.line(14,1+dy,14,23,'wood'); c.rect(14,0+dy,14,1+dy,'metal'); c.px(13,2+dy,'metal'); c.px(15,2+dy,'metal')
    legs(c,pose,7,10,19,col='gob',boot='leather',w=2)
    c.rect(6,13+dy,11,18,'gob')
    c.rect(6,17,11,19,'leather')
    c.ell(9,10+dy,4.2,3.6,'gob')
    for (x,y) in [(5,9),(4,9),(3,9),(2,8),(1,8),(0,7),(5,10),(4,10),(3,10)]: c.px(x,y+dy,'gob')   # ear
    c.px(11,9+dy,'gob_eye'); c.px(12,9+dy,'eye')
    c.px(13,11+dy,'gob'); c.px(14,11+dy,'gob')       # nose
    c.px(11,12+dy,'bone')
    c.rect(11,14+dy,13,15+dy,'gob')
    return c

def chef(pose=0, dy=0):
    c=C()
    c.rect(3,9+dy,6,20,'red')                        # cape
    legs(c,pose,6,10,19,col='gob',boot='leather',w=3)
    c.rect(5,12+dy,12,19,'gob')
    c.rect(5,16,12,19,'leather'); c.px(10,17,'gold')
    c.ell(9,8+dy,4.6,4,'gob')
    for (x,y) in [(4,8),(3,8),(2,7),(1,7),(0,6),(4,9),(3,9)]: c.px(x,y+dy,'gob')
    c.rect(6,3+dy,12,4+dy,'gold'); c.px(6,2+dy,'gold'); c.px(9,2+dy,'gold'); c.px(12,2+dy,'gold')
    c.px(11,7+dy,'gob_eye'); c.px(12,7+dy,'eye'); c.px(13,9+dy,'gob'); c.px(14,9+dy,'gob')
    c.px(10,10+dy,'bone'); c.px(12,10+dy,'bone')
    c.rect(12,13+dy,14,15+dy,'gob')
    c.line(15,8+dy,15,16+dy,'wood'); c.rect(16,6+dy,19,10+dy,'metal')
    return c


def mara(pose=0, dy=0):
    c=C()
    c.rect(3,12+dy,5,18,'pack')
    legs(c,pose,7,10,19,col='jeans',w=2)
    c.rect(6,12+dy,12,19,'jacket'); c.rect(6,18,12,18,'leather')
    c.ell(9.5,8.5+dy,4,4.2,'skin')
    for y in range(6,12): c.px(5,y+dy,'hair_br')
    c.px(6,6+dy,'hair_br'); c.px(6,10+dy,'hair_br')
    c.ell(9.5,6+dy,4.4,2.4,'cap',ymax=6+dy); c.rect(12,6+dy,15,6+dy,'cap')
    c.px(12,8+dy,'eye'); c.px(14,9+dy,'skin')
    c.line(14,18+dy,18,6+dy,'wood'); c.rect(17,4+dy,18,8+dy,'wood'); c.px(19,5+dy,'metal'); c.px(16,6+dy,'metal')
    c.rect(11,13+dy,12,16+dy,'jacket'); c.rect(13,16+dy,14,17+dy,'skin')
    return c

def zombie(pose=0, dy=0):
    c=C()
    legs(c,pose,7,10,19,col='zpants',w=2)
    c.rect(6,12+dy,11,19,'zshirt'); c.px(8,14+dy,'red'); c.px(9,15+dy,'red'); c.px(7,17,'red')
    c.ell(9.5,9+dy,4,4.2,'zskin')
    c.px(7,5+dy,'zpants'); c.px(9,5+dy,'zpants'); c.px(6,7+dy,'zpants')
    c.px(11,8+dy,'zeye'); c.px(12,8+dy,'zeye')
    c.rect(11,11+dy,12,11+dy,'red')
    c.rect(10,13+dy,18,14+dy,'zskin'); c.rect(10,13+dy,12,14+dy,'zshirt')
    c.rect(10,16+dy,16,16+dy,'zskin')
    return c

def soldat(pose=0, dy=0):
    c=C()
    c.rect(3,11+dy,5,18,'gun')
    legs(c,pose,7,10,19,col='plate',boot='gun',w=2)
    c.rect(5,12+dy,12,19,'plate'); c.rect(5,15+dy,12,15+dy,'armor')
    c.ell(9.5,7.5+dy,4.6,4.6,'plate'); c.rect(10,6+dy,14,9+dy,'visor')
    c.rect(10,13+dy,12,17+dy,'plate')
    c.rect(9,16+dy,19,17+dy,'gun'); c.rect(15,15+dy,16,15+dy,'gun'); c.px(19,15+dy,'visor')
    return c

def alien(pose=0, dy=0):
    c=C()
    c.line(1,21,6,17,'chitin'); c.px(0,22,'chitin_dk')
    legs(c,pose,7,10,19,col='chitin',boot='chitin_dk',w=2)
    c.ell(9,15+dy,3.6,4.6,'chitin')
    c.ell(10.5,8+dy,5,3,'chitin'); c.ell(6,5.5+dy,3.2,2.2,'chitin'); c.px(3,4+dy,'chitin_dk')
    c.px(13,7+dy,'acid'); c.px(14,7+dy,'acid')
    c.px(14,10+dy,'bone'); c.px(15,10+dy,'bone')
    c.line(11,13+dy,16,11+dy,'chitin'); c.px(17,10+dy,'bone'); c.px(17,12+dy,'bone')
    c.line(10,16+dy,15,15+dy,'chitin_dk'); c.px(16,14+dy,'bone')
    return c

SPRITES={'borin':(borin,False),'lyra':(lyra,False),'sef':(sef,False),'gobelin':(gobelin,True),'chef':(chef,True),'mara':(mara,False),'zombie':(zombie,True),'soldat':(soldat,False),'alien':(alien,True)}
out={}
for k,(fn,mir) in SPRITES.items():
    frames=[fn(0,0).render(mir), fn(0,1).render(mir), fn(1,0).render(mir), fn(0,1).render(mir)]
    pal,enc=encode(frames)
    out[k]={'pal':pal,'repos':[enc[0],enc[1]],'marche':[enc[2],enc[3]]}
json.dump(out,open('sprites.json','w'))
# preview
