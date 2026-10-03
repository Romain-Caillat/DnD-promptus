import json, zlib, struct, math
M=["##############",
   "#......#.....#",
   "#......#.....#",
   "#......d.....#",
   "#......#.....#",
   "###d####..~..#",
   "#............#",
   "#............#",
   "##############"]
T=16; H=T  # wall height = one tile
W=len(M[0]); R=len(M)
SP=json.load(open('sprites.json'))
def wall(x,y): return 0<=y<R and 0<=x<W and M[y][x]=='#'
def hexc(h): return tuple(int(h[i:i+2],16) for i in (1,3,5))
def noise(x,y): v=math.sin(x*12.9898+y*78.233)*43758.5453; return v-math.floor(v)
def shade(c,k): return tuple(max(0,min(255,int(v*k))) for v in c)

def render(mode):
    pad=T  # room for caps above row 0
    IW,IH=W*T,R*T+pad
    img=[[(5,5,5)]*IW for _ in range(IH)]
    def put(px,py,c):
        if 0<=px<IW and 0<=py<IH: img[py][px]=c
    def rect(x0,y0,w,h,c):
        for yy in range(y0,y0+h):
            for xx in range(x0,x0+w): put(xx,yy,c)
    # floor
    for y in range(R):
        for x in range(W):
            if M[y][x]=='#': continue
            base=(34,34,34) if M[y][x]!='~' else (24,34,40)
            for j in range(T):
                for i in range(T):
                    n=noise(x*T+i,y*T+j)
                    c=shade(base,0.9+0.2*n)
                    if i==0 or j==0: c=shade(base,1.25)
                    if i==T-1 or j==T-1: c=shade(base,.7)
                    put(x*T+i,y*T+j+pad,c)
            if M[y][x]=='d':
                rect(x*T+2,y*T+pad+2,T-4,T-4,(90,62,34)); rect(x*T+2,y*T+pad+T//2,T-4,1,(50,34,18))
    # contact shadow on floor south/east of walls
    for y in range(R):
        for x in range(W):
            if wall(x,y): continue
            if wall(x,y-1) and mode=='top':
                for j in range(5):
                    for i in range(T):
                        px,py=x*T+i,y*T+j+pad; c=img[py][px]; img[py][px]=shade(c,0.45+0.11*j)
            if wall(x,y-1) and mode=='face':
                pass
            if wall(x-1,y):
                for i in range(3):
                    for j in range(T):
                        px,py=x*T+i,y*T+j+pad; c=img[py][px]; img[py][px]=shade(c,0.6+0.13*i)
    capc=(190,190,190); rim=(235,235,235); dark=(120,120,120)
    for y in range(R):
        for x in range(W):
            if not wall(x,y): continue
            off = H if mode=='face' else 0
            cy=y*T+pad-off
            # cap: light top, rim where neighbour is not a wall
            for j in range(T):
                for i in range(T):
                    c=shade(capc,0.92+0.12*noise(x*T+i,y*T+j+7))
                    if j<2 and not wall(x,y-1): c=rim
                    if i<2 and not wall(x-1,y): c=rim
                    if i>=T-2 and not wall(x+1,y): c=dark
                    if j>=T-2 and not wall(x,y+1): c=dark
                    put(x*T+i,cy+j,c)
            if mode=='face' and not wall(x,y+1):
                # front face: brick courses, darker toward the floor
                fy=cy+T
                for j in range(H):
                    for i in range(T):
                        row=j//4; brick=((i+(row%2)*4)%8==0) or j%4==0
                        k=1.0-0.35*(j/H)
                        c=shade((96,96,96) if not brick else (58,58,58),k)
                        put(x*T+i,fy+j,c)
                # floor shadow below the face
                for j in range(4):
                    for i in range(T):
                        px,py=x*T+i,fy+H+j
                        if py<IH and not wall(x,y+1): img[py][px]=shade(img[py][px],0.5+0.12*j)
    # sprite: borin standing at (4,6), feet on the cell's lower part
    d=SP['borin']; fr=d['repos'][0]; pal=d['pal']; s=1
    bx,by=4*T-2, 6*T+pad+T-len(fr)*s
    for yy,r in enumerate(fr):
        for xx,ch in enumerate(r):
            if ch in pal: rect(bx+xx*s,by+yy*s,s,s,hexc(pal[ch]))
    # faint logical grid
    for y in range(R+1):
        for x in range(IW):
            py=y*T+pad
            if py<IH and x%2==0: img[py][x]=shade(img[py][x],1.15)
    return img

def scale(img,k):
    return [[p for p in row for _ in range(k)] for row in img for _ in range(k)]
a=scale(render('top'),3); b=scale(render('face'),3)
gap=[(20,20,20)]*24
img=[ra+gap+rb for ra,rb in zip(a,b)]
IW,IH=len(img[0]),len(img)
raw=b''.join(b'\x00'+bytes(v for p in row for v in p) for row in img)
def ch(t,d): return struct.pack('>I',len(d))+t+d+struct.pack('>I',zlib.crc32(t+d)&0xffffffff)
open('walls.png','wb').write(b'\x89PNG\r\n\x1a\n'+ch(b'IHDR',struct.pack('>IIBBBBB',IW,IH,8,2,0,0,0))+ch(b'IDAT',zlib.compress(raw,9))+ch(b'IEND',b''))
print(IW,IH)
