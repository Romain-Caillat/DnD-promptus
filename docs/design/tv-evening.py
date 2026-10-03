import json
P = 80; MX, MY = 60, 150
def cell(x, y): return MX + x * P, MY + y * P
css = r'''
body{margin:0;background:#050505}
.tv{position:relative;width:1920px;height:1080px;overflow:hidden;font-family:'Chakra Petch',system-ui,sans-serif;color:#F2F2F2;background:radial-gradient(70% 70% at 40% 50%,#1E1E1E 0,#0E0E0E 60%,#030303 100%);-webkit-font-smoothing:antialiased}
.ttl{font-family:'Cinzel',Georgia,serif;font-weight:800;letter-spacing:.03em}
.nar{font-family:'Cormorant Garamond',Georgia,serif;font-style:italic;font-weight:500}
.lbl{font-size:16px;font-weight:600;letter-spacing:.22em;text-transform:uppercase;color:#8C8C8C}
.top{position:absolute;left:60px;top:40px;display:flex;gap:18px;align-items:baseline;z-index:30}
.music{position:absolute;right:60px;top:40px;display:flex;align-items:center;gap:12px;font-size:18px;color:#A3A3A3;z-index:30}
.eq{display:flex;gap:3px;align-items:flex-end;height:20px}
.eq i{width:4px;background:#F2F2F2;animation:eq .8s ease-in-out infinite}
.eq i:nth-child(2){animation-delay:-.3s}.eq i:nth-child(3){animation-delay:-.6s}
@keyframes eq{0%,100%{height:5px}50%{height:20px}}
.center{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:24px;text-align:center}
.fog{position:absolute;inset:-10%;background:radial-gradient(30% 25% at 30% 40%,rgba(255,255,255,.05),transparent),radial-gradient(35% 30% at 70% 60%,rgba(255,255,255,.04),transparent);animation:fog 14s ease-in-out infinite alternate}
@keyframes fog{to{transform:translate(4%,-3%)}}
.seat{display:flex;flex-direction:column;align-items:center;gap:12px;opacity:.45}
.seat.on{opacity:1}
.seat .st{font-size:18px;color:#A3A3A3}
.pulse{animation:pl 1.8s ease-in-out infinite}
@keyframes pl{50%{opacity:.4}}
.line{opacity:0;animation:lin .8s ease-out forwards}
@keyframes lin{from{opacity:0;transform:translateY(12px)}to{opacity:1;transform:none}}
.art{position:absolute;inset:0;background:radial-gradient(60% 70% at 50% 100%,rgba(5,5,5,.95),transparent 70%),radial-gradient(40% 50% at 50% 45%,#3A3A3A,transparent 70%),repeating-linear-gradient(135deg,#2E2E2E 0 3px,#222 3px 11px);animation:kb 30s ease-out infinite alternate}
@keyframes kb{to{transform:scale(1.08)}}
.art .tag{position:absolute;right:60px;top:100px;font-size:14px;font-weight:700;letter-spacing:.14em;background:#0A0A0A;padding:5px 10px;border-radius:5px}
.lower{position:absolute;left:120px;right:120px;bottom:90px;display:flex;flex-direction:column;gap:14px;z-index:20}
.typed{animation:typeIn 6s steps(8) both}
@keyframes typeIn{from{clip-path:inset(0 100% 0 0)}to{clip-path:inset(0 0 0 0)}}
.dim{filter:brightness(.35)}
.who{display:flex;flex-direction:column;align-items:center;gap:10px;padding:22px 26px;border-radius:18px;background:rgba(10,10,10,.85);border:1px solid #2C2C2C;min-width:240px}
.who.done{border-color:#F2F2F2}
.who .s{font-size:20px;color:#D4D4D4}
.slam{display:inline-block;font-family:'Cinzel',serif;font-weight:800;color:#0A0A0A;background:#F2F2F2;padding:8px 30px;transform:rotate(-3deg);box-shadow:0 8px 0 #6E6E6E,0 30px 60px rgba(0,0,0,.6);animation:slam .7s cubic-bezier(.2,1.4,.3,1) both}
.slam.y{background:#FFD60A;box-shadow:0 8px 0 #8A6F00,0 30px 60px rgba(0,0,0,.6)}
@keyframes slam{0%{transform:rotate(-3deg) scale(3);opacity:0}70%{transform:rotate(-3deg) scale(.94);opacity:1}100%{transform:rotate(-3deg) scale(1)}}
.rays{position:absolute;left:50%;top:50%;width:1400px;height:1400px;margin:-700px 0 0 -700px;border-radius:50%;background:repeating-conic-gradient(rgba(255,255,255,.08) 0 7deg,transparent 7deg 20deg);-webkit-mask-image:radial-gradient(circle,#000 15%,transparent 60%);mask-image:radial-gradient(circle,#000 15%,transparent 60%);animation:spin 24s linear infinite}
@keyframes spin{to{transform:rotate(360deg)}}
.flip{animation:flip 1.4s cubic-bezier(.2,.9,.25,1.1) both;perspective:1200px}
@keyframes flip{0%{transform:rotateY(180deg) scale(.4);opacity:0}100%{transform:none;opacity:1}}
.map{position:absolute;left:60px;top:150px;width:1120px;height:800px;border-radius:20px;overflow:hidden;border:2px solid #2C2C2C;box-shadow:0 40px 90px rgba(0,0,0,.75)}
.map.shake{animation:shk 2.6s linear infinite}
@keyframes shk{0%,10%,100%{transform:none}2%{transform:translate(-10px,4px)}4%{transform:translate(10px,-4px)}6%{transform:translate(-6px,0)}8%{transform:translate(4px,2px)}}
.party{position:absolute;right:60px;top:150px;width:620px;display:flex;flex-direction:column;gap:16px}
.pc{display:flex;align-items:center;gap:20px;padding:18px 22px;border-radius:18px;background:linear-gradient(180deg,#191919,#101010);border:1px solid #2C2C2C}
.pc.now{border-color:#FFD60A;box-shadow:0 0 30px rgba(255,214,10,.25)}
.pc.hurt{border-color:#FF4D5E;animation:hurt 1.1s ease-in-out infinite}
@keyframes hurt{50%{box-shadow:0 0 34px rgba(255,77,94,.5)}}
.pf{display:inline-grid;place-items:end center;border-radius:12px;background:linear-gradient(180deg,#262626,#121212);border:2px solid #3A3A3A;box-shadow:0 4px 0 #000;padding:4px 6px 2px;flex:none}
.pf.foe{border-color:#F2F2F2;background:#0A0A0A}
.pf.now{border-color:#FFD60A;box-shadow:0 4px 0 #000,0 0 22px rgba(255,214,10,.45)}
.trk{position:absolute;left:60px;top:40px;display:flex;align-items:center;gap:0;z-index:30}
.rail{width:36px;height:3px;background:repeating-linear-gradient(90deg,#3A3A3A 0 6px,transparent 6px 12px)}
.feed{position:absolute;left:60px;bottom:40px;width:1120px;display:flex;align-items:center;gap:18px;padding:18px 24px;border-radius:16px;background:rgba(8,8,8,.92);border:1px solid #2C2C2C;font-size:26px;z-index:40;animation:lin .5s ease-out both}
.tgt{position:absolute;width:100px;height:100px;margin:-10px 0 0 -10px;border-radius:50%;border:4px dashed #FF9F1C;z-index:960;animation:spin 5s linear infinite}
.dmg{position:absolute;z-index:980;font-weight:700;font-size:88px;color:#FF4D5E;-webkit-text-stroke:5px #000;paint-order:stroke;text-shadow:0 8px 0 #000;animation:dmg 1.6s cubic-bezier(.2,.8,.2,1) both}
@keyframes dmg{0%{transform:translateY(40px) scale(.3);opacity:0}30%{transform:translateY(-20px) scale(1.4);opacity:1}45%{transform:scale(1)}100%{transform:translateY(-40px);opacity:1}}
.flash{position:absolute;inset:0;z-index:970;pointer-events:none;animation:fl 1s ease-out both}
@keyframes fl{0%{background:rgba(255,77,94,.45)}100%{background:rgba(255,77,94,0)}}
.bigdie{position:absolute;right:240px;top:600px;z-index:990;display:flex;flex-direction:column;align-items:center;gap:16px}
.res{display:flex;gap:16px;align-items:baseline;font-weight:700;font-size:64px}
.res .op{font-size:40px;color:#8C8C8C}
.loot{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);width:600px;height:700px}
.loot .beam{position:absolute;left:50%;bottom:140px;width:150px;margin-left:-75px;height:560px;border-radius:50% 50% 0 0;background:linear-gradient(90deg,transparent,rgba(255,226,140,1),transparent);filter:blur(8px);-webkit-mask-image:linear-gradient(to top,#000 30%,transparent);mask-image:linear-gradient(to top,#000 30%,transparent);animation:beam 1.2s ease-out both}
@keyframes beam{from{transform:scaleY(0);opacity:0}to{transform:none;opacity:.9}}
.loot .chest{position:absolute;left:50%;bottom:110px;margin-left:-110px;animation:chest 1s ease-in-out both}
@keyframes chest{0%{transform:rotate(-6deg)}20%{transform:rotate(6deg)}40%{transform:rotate(-4deg)}60%,100%{transform:none}}
.loot .item{position:absolute;left:50%;bottom:330px;margin-left:-110px;animation:rise 1.6s .6s cubic-bezier(.2,1.2,.3,1) both}
@keyframes rise{from{transform:translateY(200px) scale(.3);opacity:0}to{transform:none;opacity:1}}
.rec{display:flex;flex-direction:column;gap:18px;font-size:30px;line-height:1.35;text-align:left;max-width:1100px}
.next{display:flex;flex-direction:column;gap:6px;padding:20px 34px;border-radius:16px;background:#EDEDED;color:#0A0A0A;box-shadow:0 5px 0 #8A8A8A}
@media (prefers-reduced-motion: reduce){.tv *{animation:none!important}.line{opacity:1}}
'''
def party(hurt=False):
    rows = [('lyra', 'Lyra', 'Camille', '{{pvLyra}}', 22, ''), ('borin', 'Borin', 'Marc', '{{hp}}', 31, 'b'), ('sef', 'Sef', 'Hugo', '14', 16, '')]
    out = '<div class="party"><span class="lbl">Le groupe</span>'
    for p, n, who, pv, mx, b in rows:
        cls = '{{borinCls}}' if b else ''
        deg = ' degats="{{deg}}"' if b else ''
        out += f'<div class="pc {cls}"><span class="pf"><dc-import name="Sprite" perso="{p}" px="2" anim="repos" hint-size="40px,52px"></dc-import></span><div style="flex: 1; display: flex; flex-direction: column; gap: 10px"><div style="display: flex; justify-content: space-between; align-items: baseline"><span class="ttl" style="font-size: 28px">{n}</span><span style="font-size: 18px; color: #8C8C8C">{who}</span></div><div style="display: flex; align-items: center; gap: 14px"><dc-import name="Coeurs" pv="{pv}" max="{mx}" coeurs="8" px="3"{deg} hint-size="230px,24px"></dc-import><span style="font-size: 24px; font-weight: 700; color: #FF4D5E">{pv} / {mx}</span></div></div></div>'
    return out + '</div>'
def mapblock(combat, extra=''):
    c = ' combat="true"' if combat else ''
    return f'<div class="map {{{{mapCls}}}}"><div style="position: absolute; left: 0; top: -80px"><dc-import name="Plan" theme="crypte" carte="salle" pitch="80" vue="joueur" grille="true"{c} bx="{{{{bx}}}}" by="{{{{by}}}}" hint-size="1120px,800px"></dc-import></div>{extra}</div>'
def sc(n, inner): return f'<sc-if value="{{{{is{n}}}}}" hint-placeholder-val="{{{{ {"true" if n == 9 else "false"} }}}}">{inner}</sc-if>'
TOP = '<div class="top"><span class="ttl" style="font-size: 30px">La crypte des Valombre</span><span class="lbl">Session 3</span></div>'
MUSIC = '<div class="music"><span class="eq"><i></i><i></i><i></i></span>Ombres de la crypte</div>'
gx, gy = cell(5, 6); bxp, byp = cell(4, 6)
M = []
M.append(sc(1, '<div class="fog"></div><div class="center"><span class="lbl">Ce soir · 20 h 30</span><span class="ttl" style="font-size: 110px; line-height: 1">La crypte<br>des Valombre</span><span class="lbl">Session 3</span><div style="display: flex; gap: 70px; margin-top: 40px; align-items: flex-end">'
  + ''.join(f'<div class="seat on"><dc-import name="Perso" perso="{p}" px="6" etat="aucun" hint-size="120px,156px"></dc-import><span class="ttl" style="font-size: 30px">{n}</span><span class="st">{w}</span></div>' for p, n, w in [('lyra', 'Lyra', 'Camille'), ('borin', 'Borin', 'Marc'), ('sef', 'Sef', 'Hugo')])
  + '</div><span class="pulse" style="font-size: 26px; color: #A3A3A3; margin-top: 30px">En attente du MJ…</span></div>'))
M.append(sc(2, '<div class="fog"></div>' + TOP + MUSIC + '<div class="center" style="align-items: flex-start; padding: 0 200px; text-align: left; gap: 30px"><span class="ttl" style="font-size: 96px">Précédemment…</span>'
  + ''.join(f'<p class="nar line" style="margin: 0; font-size: 44px; animation-delay: {d}s">{t}</p>' for d, t in [(0.6, 'Vous êtes descendus dans la crypte des Valombre.'), (2.2, 'Les crânes regardent tous la porte nord.'), (3.8, 'Borin a pris un mauvais coup.')])
  + '<div class="line" style="display: flex; gap: 30px; margin-top: 20px; animation-delay: 5.2s"><dc-import name="Objet" objet="parchemin" rarete="commune" taille="120" hint-size="120px,120px"></dc-import><dc-import name="Objet" objet="anneau" rarete="peu" taille="120" hint-size="120px,120px"></dc-import></div></div>'))
M.append(sc(3, '<div class="art"><span class="tag">[ILLUSTRATION · SALLE DE L’AUTEL]</span></div>' + TOP + MUSIC + '<div class="lower"><span class="ttl" style="font-size: 64px">La salle de l’autel</span><p class="nar typed" style="margin: 0; font-size: 40px; color: #E5E5E5">L’escalier s’arrête devant un autel de pierre noire. Des cendres encore tièdes.</p></div>'))
M.append(sc(4, '<div class="art dim"></div>' + TOP + MUSIC + '<div class="center"><p class="nar" style="margin: 0; font-size: 72px">« Devant l’autel, que faites-vous ? »</p><div style="display: flex; gap: 30px; margin-top: 40px">'
  + '<div class="who done"><dc-import name="Sprite" perso="lyra" px="3" anim="repos" hint-size="60px,78px"></dc-import><span class="ttl" style="font-size: 26px">Lyra</span><span class="s">surveille l’escalier</span></div>'
  + '<div class="who {{borinWho}}"><dc-import name="Sprite" perso="borin" px="3" anim="repos" hint-size="60px,78px"></dc-import><span class="ttl" style="font-size: 26px">Borin</span><span class="s">{{borinProp}}</span></div>'
  + '<div class="who"><dc-import name="Sprite" perso="sef" px="3" anim="repos" hint-size="60px,78px"></dc-import><span class="ttl" style="font-size: 26px">Sef</span><span class="s pulse">réfléchit…</span></div></div></div>'))
M.append(sc(5, '<div class="art dim"></div>' + TOP + MUSIC + '<div class="center"><span class="lbl">Borin</span><span class="ttl" style="font-size: 64px">{{diceTitle}}</span><span style="font-size: 32px; color: #D4D4D4">Il faut <b style="color: #F2F2F2">13 ou plus</b></span>'
  + '<sc-if value="{{diceIdle}}" hint-placeholder-val="{{ true }}"><dc-import name="De" de="d20" valeur="20" taille="320" anim="fixe" hint-size="320px,320px"></dc-import><span class="pulse" style="font-size: 26px; color: #A3A3A3">Marc lance le dé…</span></sc-if>'
  + '<sc-if value="{{diceRolling}}" hint-placeholder-val="{{ false }}"><dc-import name="De" de="d20" valeur="16" taille="320" anim="roule" hint-size="320px,320px"></dc-import></sc-if>'
  + '<sc-if value="{{diceDone}}" hint-placeholder-val="{{ false }}"><dc-import name="De" de="d20" valeur="16" taille="320" anim="fixe" hint-size="320px,320px"></dc-import><div class="res"><span>16</span><span class="op">+ 4</span><span class="op">=</span><span>20</span></div><span class="slam" style="font-size: 64px">RÉUSSI</span></sc-if></div>'))
M.append(sc(6, TOP + MUSIC + '<div class="rays"></div><div class="center"><span class="lbl">Indice trouvé par Borin</span><div class="flip"><dc-import name="GameCard" titre="La page arrachée" type="Indice" texte="Coincée sous l’autel : la page qui manquait au registre." valeur="" stat="none" icon="parchemin" w="360" hint-size="360px,504px"></dc-import></div></div>'))
M.append(sc(7, TOP + MUSIC + mapblock(False) + party() + '<div class="feed"><span class="lbl">Exploration</span><span>{{exploreTxt}}</span></div>'))
TRK = '<div class="trk" style="left: 1240px; top: 40px">' + '<span class="rail"></span>'.join(f'<span class="pf {c}"><dc-import name="Sprite" perso="{p}" px="2" anim="repos" hint-size="40px,52px"></dc-import></span>' for p, c in [('gobelin', '{{trk0}}'), ('sef', '{{trk1}}'), ('borin', '{{trk2}}'), ('lyra', ''), ('gobelin', 'foe')]) + '</div>'
M.append(sc(8, TOP + TRK + mapblock(True) + party() + '<sc-if value="{{feed0}}" hint-placeholder-val="{{ true }}"><div class="center" style="pointer-events: none; z-index: 990"><span class="slam y" style="font-size: 120px">COMBAT !</span></div></sc-if><div class="feed"><span class="lbl">Round 1</span><span>{{feedTxt}}</span></div>'))
M.append(sc(9, TOP + TRK + mapblock(True, f'<span class="tgt" style="left: {gx - MX}px; top: {gy - MY}px"></span><sc-if value="{{{{atkHit}}}}" hint-placeholder-val="{{{{ false }}}}"><span class="dmg" style="left: {gx - MX + 10}px; top: {gy - MY - 80}px">−9</span></sc-if>') + party()
  + '<sc-if value="{{atkRolling}}" hint-placeholder-val="{{ false }}"><div class="bigdie"><dc-import name="De" de="d20" valeur="17" taille="260" anim="roule" couleur="atq" hint-size="260px,260px"></dc-import></div></sc-if>'
  + '<div class="feed"><span class="lbl">Tour de Borin</span><span>{{atkTxt}}</span></div>'))
M.append(sc(10, TOP + TRK + mapblock(True, f'<div class="flash"></div><span class="dmg" style="left: {bxp - MX}px; top: {byp - MY - 80}px">−6</span>') + party() + '<div class="feed"><span class="lbl">Le gobelin riposte</span><span>Sa lame passe sous le bouclier de Borin : <b style="color: #FF4D5E">6 dégâts</b>.</span></div>'))
M.append(sc(11, TOP + MUSIC + '<div class="rays"></div><div class="loot"><div class="beam"></div><div class="chest"><dc-import name="Objet" objet="coffre" cadre="false" taille="220" hint-size="220px,220px"></dc-import></div><div class="item"><dc-import name="Objet" objet="hache" rarete="leg" taille="220" hint-size="220px,220px"></dc-import></div></div><div class="lower" style="align-items: center; text-align: center"><span class="lbl">Borin reçoit</span><span class="ttl" style="font-size: 72px; color: #F3CC63">Hache des Valombre</span><span class="lbl" style="color: #F3CC63">Légendaire · {{lootState}}</span></div>'))
M.append(sc(12, '<div class="fog"></div>' + TOP + '<div class="center" style="gap: 40px"><span class="ttl" style="font-size: 96px">Ce soir</span><div class="rec"><span>— Vous avez trouvé la page arrachée du registre.</span><span>— Vous avez vaincu les gobelins de la crypte.</span><span>— La porte nord est ouverte. Quelque chose chante derrière.</span></div><div class="next"><span class="lbl" style="color: #5A5A5A">Prochaine session</span><span class="ttl" style="font-size: 40px">Jeudi 10 octobre · 20 h 30</span></div></div>'))
JS = r'''
class Component extends DCLogic {
  renderVals() {
    const p = this.props;
    const m = Number(p.moment ?? 9);
    const dice = p.dice ?? 'idle', atk = p.atk ?? 'idle', card = Number(p.card ?? 0), feed = Number(p.feed ?? 0);
    const choice = Number(p.choice ?? 0), sent = String(p.sent ?? false) === 'true', moved = String(p.moved ?? false) === 'true';
    const hp = Number(p.hp ?? (m >= 10 ? 9 : 15)), deg = Number(p.deg ?? (m === 10 ? 6 : 0));
    const opts = ['Fouiller l’autel', 'Lire les inscriptions', 'Parler aux autres', 'Son idée à lui'];
    const v = {
      hp, deg, pvLyra: m >= 9 ? 18 : 22,
      bx: Number(p.bx ?? 4), by: Number(p.by ?? 6),
      borinWho: sent ? 'done' : '', borinProp: sent ? 'propose : ' + opts[choice].toLowerCase() : 'réfléchit…',
      diceTitle: choice === 3 ? 'Son idée' : opts[choice], diceIdle: dice === 'idle', diceRolling: dice === 'rolling', diceDone: dice === 'done',
      exploreTxt: moved ? 'Borin avance vers la porte de droite… quelque chose bouge.' : 'Le groupe peut avancer. Borin choisit où aller.',
      feed0: m === 8 && feed === 0, feedTxt: ['Des gobelins surgissent ! Le premier tire sur Lyra…', 'Le gobelin tire sur Lyra… raté !', 'Sef lance une dague : touché, 4 dégâts.'][feed],
      trk0: m === 8 && feed < 2 ? 'foe now' : 'foe', trk1: m === 8 && feed === 2 ? 'now' : '', trk2: m >= 9 ? 'now' : '',
      atkRolling: atk === 'rolling' || atk === 'asking', atkHit: m === 9 && atk === 'done' && card === 0,
      atkTxt: atk === 'idle' ? 'Borin choisit son action…' : atk === 'rolling' ? 'Borin frappe à la hache… il faut 13.' : atk === 'asking' ? 'Borin propose une idée au MJ…' : card === 0 ? 'Touché ! 17 + 5 = 22. La hache fait 9 dégâts.' : card === 1 ? 'Borin reprend son souffle : +7 PV.' : card === 2 ? 'Borin entre en rage : +2 dégâts.' : 'Borin renverse la table : le gobelin tombe à terre !',
      borinCls: m === 10 ? 'hurt' : m === 9 ? 'now' : '',
      mapCls: m === 10 ? 'shake' : '',
      lootState: p.equipped === 'equip' ? 'équipée' : p.equipped === 'keep' ? 'dans le sac' : 'à toi, Marc'
    };
    for (let i = 1; i <= 12; i++) v['is' + i] = m === i;
    return v;
  }
}
'''
html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Écran TV · soirée</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700;800&amp;family=Chakra+Petch:wght@400;500;600;700&amp;family=Cormorant+Garamond:ital,wght@1,500;1,600&amp;display=swap">
<style>{css}</style>
</helmet>
<div class="tv">{''.join(M)}</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"moment":{{"editor":"int","default":9,"min":1,"max":12}},"$preview":{{"width":1920,"height":1080}}}}'>{JS}</script>
</body>
</html>
'''
open('docs/design/canvas/Ecran-TV.dc.html', 'w').write(html)
print(len(html))

import sys as _s; _s.path.insert(0, 'gen')
from scope import scope
_h = open('docs/design/canvas/Ecran-TV.dc.html').read()
open('docs/design/canvas/Ecran-TV.dc.html', 'w').write(scope(_h, 'tv-')[0])
