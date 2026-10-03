import json, zlib, struct
I = {
 "epee": {"pal": {"L":"#F2F2F2","M":"#A9B1BA","G":"#E0A82E","B":"#7A4A22"}, "px": [
  "..........LL",
  ".........LLM",
  "........LLM.",
  ".......LLM..",
  "......LLM...",
  ".....LLM....",
  "..G.LLM.....",
  "...GLM......",
  "...BGG......",
  "..BB..G.....",
  ".BB.........",
  "GB.........."]},
 "hache": {"pal": {"L":"#E6E9EC","M":"#8E969F","B":"#8A5A2B","b":"#5C3A18","G":"#E0A82E"}, "px": [
  ".M....Bb....",
  "MLM...Bb....",
  "MLLM..Bb....",
  "MLLLMMBbM...",
  "MLLLLLBbMM..",
  "MLLLMMBbM...",
  "MLLM..Bb....",
  "MLM...Bb....",
  ".M....Bb....",
  "......Bb....",
  "......Bb....",
  "......GG...."]},
 "potion": {"pal": {"C":"#8A5A2B","W":"#D8E4EA","G":"#9FB3BD","R":"#FF4D5E","r":"#C22A3C","h":"#FFFFFF"}, "px": [
  ".....CC.....",
  ".....CC.....",
  "....WGGW....",
  ".....GG.....",
  "....WGGW....",
  "...WRRRRW...",
  "..WRhRRRRW..",
  "..WRhRRRRW..",
  "..WRRRRRrW..",
  "..WRRRRrrW..",
  "...WRRRrW...",
  "....WWWW...."]},
 "bouclier": {"pal": {"M":"#A9B1BA","L":"#E6E9EC","B":"#8A5A2B","b":"#6A4220"}, "px": [
  ".MMMMMMMMMM.",
  ".MBBBLLBBBM.",
  ".MBBBLLBBbM.",
  ".MBBBLLBBbM.",
  ".MLLLLLLLLM.",
  ".MBBBLLBBbM.",
  ".MBBBLLBBbM.",
  "..MBBLLBbM..",
  "..MBBLLBbM..",
  "...MBLLbM...",
  "....MLLM....",
  ".....MM....."]},
 "anneau": {"pal": {"V":"#B28CFF","v":"#7A55D0","h":"#FFFFFF","G":"#F3CC63","g":"#B07A18"}, "px": [
  "............",
  ".....VV.....",
  "....VhVV....",
  "....VVVv....",
  ".....GG.....",
  "...GG..GG...",
  "..G......G..",
  "..G......g..",
  "..g......g..",
  "...gg..gg...",
  ".....gg.....",
  "............"]},
 "parchemin": {"pal": {"P":"#EFE2C0","p":"#C9B48A","k":"#6E5A3A","R":"#C22A3C"}, "px": [
  "............",
  ".pPPPPPPPPp.",
  "pPPPPPPPPPPp",
  ".pPPPPPPPPp.",
  "..PkkkkkkP..",
  "..PPPPPPPP..",
  "..PkkkkPPP..",
  "..PPPPPPRR..",
  "..PkkkkPRR..",
  ".pPPPPPPPPp.",
  "pPPPPPPPPPPp",
  ".pPPPPPPPPp."]},
 "couronne": {"pal": {"G":"#F3CC63","g":"#B07A18","h":"#FFF7D6","R":"#FF4D5E","B":"#3D9BFF"}, "px": [
  "............",
  "............",
  ".h...hh...h.",
  ".G...GG...G.",
  ".GG.GGGG.GG.",
  ".GGGGGGGGGG.",
  ".GhGGGGGGGG.",
  ".GRGGBBGGRG.",
  ".GGGGGGGGGG.",
  ".gggggggggg.",
  "............",
  "............"]},
 "orbe": {"pal": {"W":"#FFFFFF","A":"#BDF4FF","a":"#7FD8F0","h":"#FFFFFF","G":"#F3CC63","g":"#B07A18"}, "px": [
  "............",
  "....WWWW....",
  "...WAAAAW...",
  "..WAhhAAAW..",
  "..WAhAAAAW..",
  "..WAAAAAaW..",
  "..WAAAAaaW..",
  "...WAAaaW...",
  "....WWWW....",
  "....gGGg....",
  "...gGGGGg...",
  "..gggggggg.."]},
 "bourse": {"pal": {"B":"#9A6A3A","b":"#6A4220","h":"#C99A6A","G":"#F3CC63","k":"#4A2E12"}, "px": [
  "............",
  "....k..k....",
  ".....kk.....",
  ".....GG.....",
  "....BBBB....",
  "...BBhBBB...",
  "..BBhBBBBB..",
  "..BBBBBBbB..",
  "..BBBBBbbB..",
  "..BBBBbbbB..",
  "...BBBbbB...",
  "....BBBB...."]},
 "coffre": {"pal": {"B":"#9A6A3A","b":"#6A4220","G":"#E0A82E","k":"#2A1A08"}, "px": [
  ".BBBBBBBBBB.",
  "BbBBBBBBBBbB",
  "BbBBBBBBBBbB",
  "GGGGGGGGGGGG",
  "BBBBBGGBBBBB",
  "BbBBBGkBBBbB",
  "BbBBBBBBBBbB",
  "BbBBBBBBBBbB",
  "BbBBBBBBBBbB",
  "GGGGGGGGGGGG"]}
}
json.dump(I, open('items.json','w'), ensure_ascii=False)
S=8; names=list(I); CW=16
IW=CW*S*len(names); IH=CW*S
pix=[[(28,28,28)]*IW for _ in range(IH)]
for i,k in enumerate(names):
    g=I[k]['px']; pal=I[k]['pal']
    at=lambda x,y: 0<=y<len(g) and 0<=x<len(g[0]) and g[y][x]!='.'
    for y in range(-1,len(g)+1):
        for x in range(-1,13):
            if at(x,y): h=pal[g[y][x]]
            elif at(x+1,y) or at(x-1,y) or at(x,y+1) or at(x,y-1): h='#050505'
            else: continue
            col=tuple(int(h[n:n+2],16) for n in (1,3,5))
            for b in range(S):
                row=pix[(y+2)*S+b]
                for a in range(S): row[(i*CW+x+2)*S+a]=col
raw=b''.join(b'\x00'+bytes(v for p in row for v in p) for row in pix)
def chunk(t,data): return struct.pack('>I',len(data))+t+data+struct.pack('>I',zlib.crc32(t+data)&0xffffffff)
open('items.png','wb').write(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',IW,IH,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(raw,9))+chunk(b'IEND',b''))
for k,v in I.items():
    assert all(len(r)==12 for r in v['px']),k
    assert all(c=='.' or c in v['pal'] for r in v['px'] for c in r),k
print('ok')
