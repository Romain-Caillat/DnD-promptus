"""Generate the playable GM board: Marc's evening seen from the GM laptop (Mener-MJ.dc.html).

Same twelve moments as Jouer-Marc and Ecran-TV. The GM's main action moves the evening on;
the players' side (Marc's choices, rolls, moves) is simulated with timers. The TV beside the
laptop is the Ecran-TV component, driven from this board's state.
Run from the repository root.
"""
import json

P = 48  # map pitch on the GM screen
css = r'''
body{margin:0;background:#0A0A0A}
.pm{font-family:'Chakra Petch',system-ui,sans-serif;color:#F2F2F2;background:radial-gradient(120% 60% at 50% 0%,#1C1C1C 0,#0C0C0C 55%,#050505 100%);-webkit-font-smoothing:antialiased}
.ttl{font-family:'Cinzel',Georgia,serif;font-weight:800;letter-spacing:.02em}
.nar{margin:0;font-family:'Cormorant Garamond',Georgia,serif;font-style:italic;font-weight:500;line-height:1.35;color:#E5E5E5}
.lbl{font-size:10px;font-weight:600;letter-spacing:.2em;text-transform:uppercase;color:#8C8C8C}
.lap{position:relative;width:1440px;height:900px;flex:none;overflow:hidden;box-sizing:border-box;display:grid;grid-template-rows:52px 1fr;background:radial-gradient(90% 70% at 50% 0%,#1A1A1A 0,#0C0C0C 60%,#050505 100%);border-radius:14px;box-shadow:0 0 0 2px #2C2C2C,0 30px 60px rgba(0,0,0,.7)}
.top{display:flex;align-items:center;gap:18px;padding:0 18px;border-bottom:1px solid #1F1F1F;font-size:12px;color:#A3A3A3;white-space:nowrap}
.top .sep{width:1px;height:20px;background:#2A2A2A;flex:none}
.pres{width:8px;height:8px;border-radius:50%;background:#F2F2F2;flex:none;box-shadow:0 0 8px rgba(255,255,255,.6)}
.tvnow{display:flex;align-items:center;gap:8px;padding:5px 10px;border-radius:8px;background:#141414;border:1px solid #2C2C2C;color:#D4D4D4}
.tvnow > b{color:#F2F2F2;font-weight:600}
.yt{display:flex;align-items:center;gap:10px;padding:4px 10px 4px 4px;border-radius:8px;background:#141414;border:1px solid #2C2C2C}
.yt .thumb{width:44px;height:28px;border-radius:4px;background:repeating-linear-gradient(135deg,#3A3A3A 0 2px,#262626 2px 7px);display:grid;place-items:center;flex:none}
.eq{display:flex;gap:2px;align-items:flex-end;height:14px}
.eq > i{width:3px;height:6px;background:#F2F2F2;animation:eq .8s ease-in-out infinite}
.eq > i:nth-child(2){animation-delay:-.3s}.eq > i:nth-child(3){animation-delay:-.6s}
@keyframes eq{0%,100%{height:4px}50%{height:14px}}
.cols{display:grid;grid-template-columns:272px 728px 384px;gap:14px;padding:14px 14px 14px;min-height:0}
.col{display:flex;flex-direction:column;gap:14px;min-height:0;overflow:hidden}
.panel{background:linear-gradient(180deg,#181818,#111);border:1px solid #2C2C2C;border-radius:14px;box-shadow:inset 0 1px 0 rgba(255,255,255,.06),0 12px 30px rgba(0,0,0,.45)}
.ph{display:flex;justify-content:space-between;align-items:baseline;padding:12px 14px 8px}
.track{display:flex;flex-direction:column;padding:0 10px 10px}
.st{display:flex;align-items:center;gap:10px;height:30px;padding:0 8px;border-radius:8px;font-size:13px;color:#6E6E6E}
.st > i{width:10px;height:10px;border-radius:3px;border:1.5px solid #3A3A3A;flex:none;box-sizing:border-box}
.st.done{color:#A3A3A3}
.st.done > i{background:#5A5A5A;border-color:#5A5A5A}
.st.cur{background:#EDEDED;color:#0A0A0A;font-weight:700;box-shadow:0 3px 0 #8A8A8A}
.st.cur > i{background:#0A0A0A;border-color:#0A0A0A}
.st > small{margin-left:auto;font-size:11px;font-weight:600;opacity:.75}
.rows{display:flex;flex-direction:column;gap:4px;padding:0 10px 10px}
.row{display:flex;align-items:center;gap:10px;height:62px;box-sizing:border-box;padding:0 8px;border-radius:10px;border:1px solid transparent}
.row.now{background:#1E1E1E;border-color:#FFD60A;box-shadow:0 0 18px rgba(255,214,10,.15)}
.row .nm{display:flex;justify-content:space-between;gap:6px;font-size:13px;font-weight:600}
.row .nm > small{font-size:11px;font-weight:500;color:#8C8C8C}
.row .sub{font-size:11px;color:#A3A3A3;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.row .ini{width:18px;font-size:12px;font-weight:700;color:#FFD60A;flex:none}
.row .hp{font-size:12px;font-weight:700;color:#FF4D5E;flex:none}
.pf{display:inline-grid;place-items:end center;border-radius:8px;background:linear-gradient(180deg,#262626,#121212);border:2px solid #3A3A3A;box-shadow:0 3px 0 #000;padding:2px 3px 0;flex:none}
.pf.foe{border-color:#F2F2F2;background:#0A0A0A}
.front{display:flex;align-items:center;gap:12px;padding:0 14px 14px}
.front > p{margin:0;font-size:12px;line-height:1.4;color:#A3A3A3}
.front > p > b{display:block;font-size:13px;color:#F2F2F2}
.bn{display:flex;align-items:center;gap:10px;height:44px;padding:0 16px;border-radius:12px;font-weight:700;font-size:15px;box-sizing:border-box;flex:none}
.bn.you{background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.bn.wait{background:#0A0A0A;border:1.5px dashed #5E5E5E;color:#D4D4D4}
.bn.live{background:#161616;border:1px solid #2C2C2C;color:#D4D4D4}
.bn.turn{background:#FFD60A;color:#0A0A0A;box-shadow:0 3px 0 #8A6F00}
.stage{flex:1;min-height:0;display:flex;flex-direction:column;gap:14px;padding:16px;overflow:hidden}
.foot{flex:none;display:flex;gap:10px;align-items:stretch;padding-bottom:8px}
.go{font:inherit;flex:1;display:block;box-sizing:border-box;border:0;padding:6px;border-radius:12px;background:#EDEDED;color:#0A0A0A;text-align:left;cursor:pointer;box-shadow:0 4px 0 #8A8A8A,0 7px 0 #5A5A5A,0 14px 24px rgba(0,0,0,.5);transform-origin:50% 100%;transition:transform .1s,box-shadow .1s}
.go .in{display:flex;align-items:center;gap:12px;border:1.5px solid #0A0A0A;border-radius:8px;padding:6px 14px;height:42px;box-sizing:border-box}
.go .t{display:block;font-family:'Cinzel',Georgia,serif;font-weight:800;font-size:17px;line-height:1.05}
.go .s{display:block;font-size:11px;font-weight:600;color:#5A5A5A;margin-top:2px}
.go .chev{margin-left:auto;font-family:'Cinzel',serif;font-weight:800;font-size:22px}
.go:active{transform:perspective(500px) rotateX(14deg) translateY(4px);box-shadow:0 1px 0 #8A8A8A,0 2px 0 #5A5A5A}
.go.off{background:#141414;color:#8C8C8C;box-shadow:0 0 0 1.5px #2C2C2C;cursor:default;pointer-events:none}
.go.off .in{border:1.5px dashed #3A3A3A}
.go.off .s{color:#6E6E6E}
.btn{font:inherit;font-family:'Cinzel',Georgia,serif;font-size:12px;font-weight:800;color:#F2F2F2;background:#141414;border:0;border-radius:8px;height:34px;padding:0 12px;display:inline-flex;align-items:center;justify-content:center;gap:6px;cursor:pointer;white-space:nowrap;outline:1.5px solid #5A5A5A;outline-offset:-4px;box-shadow:0 0 0 1.5px #3A3A3A,0 3px 0 #000;flex:none}
.btn:active{transform:translateY(2px);box-shadow:0 0 0 1.5px #3A3A3A,0 1px 0 #000}
.sec{display:flex;flex-direction:column;gap:8px}
.hero{display:flex;flex-direction:column;gap:6px}
.seats{display:grid;grid-template-columns:repeat(3,1fr);gap:12px}
.seat{display:flex;flex-direction:column;align-items:center;gap:6px;padding:14px 10px;border-radius:14px;background:radial-gradient(60% 30% at 50% 92%,rgba(255,255,255,.08),transparent),#141414;border:1px solid #2C2C2C;font-size:12px;color:#A3A3A3}
.seat > b{font-family:'Cinzel',serif;font-size:18px;color:#F2F2F2}
.chk{display:flex;align-items:center;gap:10px;font-size:13px;padding:9px 12px;border-radius:10px;background:#121212}
.chk > i{width:18px;height:18px;border-radius:5px;background:#EDEDED;color:#0A0A0A;display:grid;place-items:center;font-style:normal;font-weight:800;font-size:12px;flex:none}
.chk > i.pend{background:transparent;border:1.5px dashed #5E5E5E;color:#8C8C8C;box-sizing:border-box}
.chk > span:last-child{margin-left:auto;font-size:11px;color:#8C8C8C}
.edit{position:relative;padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1px solid #333}
.edit .lbl{display:block;margin-bottom:8px}
.caret{display:inline-block;width:2px;height:18px;background:#F5F5F5;margin-left:2px;vertical-align:-3px;animation:caret .8s steps(1) infinite}
@keyframes caret{50%{opacity:0}}
.clue{display:flex;align-items:center;gap:12px;font-size:13px;padding:8px;border-radius:10px;background:#141414;border:1px solid #2C2C2C}
.clue > em{margin-left:auto;font-style:normal;font-size:11px;color:#8C8C8C;white-space:nowrap}
.clue.key{border-color:#F2F2F2}
.art{height:178px;border-radius:12px;position:relative;background:radial-gradient(70% 80% at 50% 100%,rgba(10,10,10,.9),transparent),repeating-linear-gradient(135deg,#3A3A3A 0 2px,#2A2A2A 2px 9px);flex:none}
.tag{position:absolute;right:10px;top:10px;font-size:9px;font-weight:700;letter-spacing:.14em;background:#0A0A0A;padding:3px 6px;border-radius:4px}
.secret{display:flex;flex-direction:column;gap:6px;padding:12px 14px;border-radius:12px;background:#050505;border:1.5px dashed #5E5E5E;font-size:13px;line-height:1.5;color:#D4D4D4}
.secret .lbl{color:#F2F2F2}
.exits{display:flex;gap:8px}
.exit{font-size:12px;padding:6px 10px;border-radius:999px;background:#141414;border:1px solid #333;color:#D4D4D4;white-space:nowrap}
.ans{display:flex;align-items:center;gap:12px;padding:10px 12px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:13px}
.ans .who{display:flex;flex-direction:column;gap:2px;width:96px;flex:none}
.ans .who > b{font-family:'Cinzel',serif;font-size:15px}
.ans .who > small{font-size:11px;color:#8C8C8C}
.ans > p{margin:0;flex:1;color:#D4D4D4}
.ans .ok{font-size:11px;font-weight:700;letter-spacing:.08em;text-transform:uppercase;color:#A3A3A3;white-space:nowrap}
.req{display:flex;flex-direction:column;gap:10px;padding:12px 14px;border-radius:12px;background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A,0 10px 20px rgba(0,0,0,.4);animation:reqIn .6s cubic-bezier(.2,1.2,.3,1) both}
@keyframes reqIn{0%{opacity:0;transform:translateX(30px) rotate(3deg)}100%{opacity:1;transform:none}}
.req .hd{display:flex;align-items:center;gap:12px}
.req .hd > div{display:flex;flex-direction:column;gap:2px;flex:1}
.req .hd > div > b{font-family:'Cinzel',serif;font-size:17px}
.req .hd > div > small{font-size:12px;color:#5A5A5A}
.req .lbl{color:#5A5A5A}
.req .acts{display:flex;gap:8px}
.pulse{animation:pulse 1.6s ease-in-out infinite}
@keyframes pulse{50%{opacity:.45}}
.dice{display:grid;grid-template-columns:240px 1fr;gap:20px;align-items:center}
.dz{display:flex;flex-direction:column;align-items:center;gap:10px;min-height:250px;justify-content:center}
.res{display:flex;gap:10px;align-items:baseline;font-weight:700;font-size:30px}
.res .op{font-size:20px;color:#8C8C8C}
.win{font-family:'Cinzel',serif;font-weight:800;font-size:24px;color:#0A0A0A;background:#F2F2F2;padding:3px 16px;border-radius:8px;box-shadow:0 4px 0 #6E6E6E}
.out{display:flex;flex-direction:column;gap:4px;padding:10px 12px;border-radius:10px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.45;color:#D4D4D4}
.out.on{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.out.on .lbl{color:#5A5A5A}
.flip{animation:flipIn 1.4s cubic-bezier(.2,.9,.25,1.1) both;perspective:900px}
@keyframes flipIn{0%{transform:rotateY(180deg) scale(.6);opacity:0}100%{transform:none;opacity:1}}
.mapw{position:relative;isolation:isolate;width:672px;height:432px;overflow:hidden;border-radius:12px;border:1px solid #2C2C2C;flex:none;align-self:center}
.fogz{position:absolute;z-index:940;border:2px dashed #F2F2F2;border-radius:6px;background:rgba(5,5,5,.25);pointer-events:none}
.fogz > span{position:absolute;left:8px;bottom:8px;font-size:10px;font-weight:700;letter-spacing:.12em;text-transform:uppercase;background:#0A0A0A;padding:3px 6px;border-radius:4px;white-space:nowrap}
.tgt{position:absolute;width:56px;height:56px;margin:-4px 0 0 -4px;border-radius:50%;border:3px dashed #FF9F1C;z-index:960;box-sizing:border-box;animation:spin 5s linear infinite}
@keyframes spin{to{transform:rotate(360deg)}}
.dmg{position:absolute;z-index:980;font-weight:700;font-size:44px;line-height:1;color:#FF4D5E;-webkit-text-stroke:3px #000;paint-order:stroke;text-shadow:0 4px 0 #000}
.tools{display:flex;gap:8px;align-items:center}
.tools .lbl{margin-left:auto}
.line{font-size:13px;color:#D4D4D4;line-height:1.45}
.line > b{color:#F2F2F2}
.two{display:grid;grid-template-columns:1fr 1fr;gap:14px;min-height:0}
.loot{display:flex;align-items:center;gap:24px;padding:18px;border-radius:14px;background:radial-gradient(50% 60% at 18% 60%,rgba(255,226,140,.18),transparent),#141414;border:1px solid #2C2C2C}
.co{display:flex;flex-direction:column;gap:10px;padding:0 14px 14px}
.ai{display:flex;align-items:center;gap:8px}
.ai > span:last-child{margin-left:auto;font-size:11px;color:#8C8C8C}
.sugg{display:flex;align-items:center;gap:10px;font-size:12px;padding:7px 8px 7px 10px;border-radius:8px;background:#161616}
.sugg > code{font-family:'Chakra Petch',monospace;font-size:11px;color:#8A8A8A;flex:none}
.sugg > span{flex:1;min-width:0}
.jr{display:flex;flex-direction:column;gap:2px;padding:0 14px 14px;overflow:hidden;min-height:0}
.ev{display:flex;gap:10px;font-size:12px;line-height:1.4;padding:6px 0;border-bottom:1px solid #1C1C1C;color:#D4D4D4}
.ev > time{font-size:11px;color:#6E6E6E;flex:none;width:36px}
.ev.key{color:#F2F2F2;font-weight:600}
.ev.hid > em{font-style:normal;color:#FF9F1C}
.endc{position:absolute;inset:0;z-index:70;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:16px;text-align:center;background:rgba(5,5,5,.94)}
.side{display:flex;flex-direction:column;gap:18px;width:640px;flex:none}
.tvf{width:640px;height:360px;border-radius:10px;overflow:hidden;border:1px solid #2C2C2C;box-shadow:0 20px 40px rgba(0,0,0,.6);flex:none}
.think{position:relative;padding:14px 16px;border-radius:14px;background:#EDEDED;color:#0A0A0A;font-size:14px;line-height:1.45;box-shadow:0 3px 0 #8A8A8A;min-height:72px;box-sizing:border-box;display:flex;align-items:center}
.seen{display:flex;gap:10px;align-items:flex-start;padding:12px 14px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.45;color:#D4D4D4;min-height:46px;box-sizing:border-box}
.prog{display:grid;grid-template-columns:repeat(12,1fr);gap:6px}
.prog > button{font:inherit;font-size:13px;font-weight:700;height:36px;border-radius:8px;background:#141414;border:1px solid #2C2C2C;color:#6E6E6E;cursor:pointer}
.prog > button.done{color:#A3A3A3}
.prog > button.cur{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.pick{display:flex;gap:8px}
.pick > button{font:inherit;flex:1;font-size:13px;font-weight:600;height:38px;border-radius:9px;background:#141414;border:1px solid #2C2C2C;color:#A3A3A3;cursor:pointer}
.pick > button.on{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.ctl{display:flex;gap:10px}
.ctl > button{font:inherit;font-size:13px;font-weight:600;height:40px;padding:0 16px;border-radius:10px;background:#141414;border:1px solid #3A3A3A;color:#D4D4D4;cursor:pointer}
@media (prefers-reduced-motion: reduce){.pm *{animation:none!important}}
'''


def go():
    return '<div class="foot"><button type="button" class="go {{mainCls}}" onClick="{{mainAct}}"><span class="in"><span style="flex: 1"><span class="t">{{mainLbl}}</span><span class="s">{{mainSub}}</span></span><span class="chev">›</span></span></button></div>'


def block(n, body):
    return f'<sc-if value="{{{{is{n}}}}}" hint-placeholder-val="{{{{ {"true" if n == 1 else "false"} }}}}">{body}</sc-if>'


def imp(name, size, **kw):
    a = ' '.join(f'{k}="{v}"' for k, v in kw.items())
    return f'<dc-import name="{name}" {a} hint-size="{size[0]}px,{size[1]}px"></dc-import>'


def ttl(lbl, title, extra=''):
    return f'<div style="display: flex; align-items: baseline; gap: 14px"><span class="ttl" style="font-size: 24px">{title}</span><span class="lbl">{lbl}</span>{extra}</div>'


def plan(combat, overlay=''):
    c = ' combat="true"' if combat else ''
    return (f'<div class="mapw"><div style="position: absolute; left: 0; top: -{P}px"><dc-import name="Plan" theme="crypte" carte="salle" pitch="{P}" vue="mj" grille="true"{c} '
            f'bx="{{{{bx}}}}" by="{{{{by}}}}" hint-size="{14 * P}px,{10 * P}px"></dc-import></div>{overlay}</div>')


def at(x, y):
    """Top-left of a cell inside the cropped map window (the first map row is cropped off)."""
    return x * P, y * P


B = []
# 1 · the lobby
B.append(block(1, '<div class="hero"><span class="lbl">Ce soir · 20 h 30</span><span class="ttl" style="font-size: 34px; line-height: 1.1">La crypte des Valombre</span><span style="font-size: 13px; color: #A3A3A3">Session 3 · on reprend dans la salle de l’autel</span></div>'
  + '<div class="seats">'
  + ''.join(f'<div class="seat">{imp("Perso", (80, 104), perso=p, px=4, etat="aucun")}<b>{n}</b><span>{w}</span><span style="color: #D4D4D4">{s}</span></div>' for p, n, w, s in [
      ('lyra', 'Lyra', 'Camille · connectée', 'son activé'), ('borin', 'Borin', 'Marc · connecté', '{{marcSound}}'), ('sef', 'Sef', 'Hugo · connecté', 'son activé')])
  + '</div><div class="sec"><span class="lbl">Avant de lancer</span>'
  + '<div class="chk"><i>✓</i><span>Écran partagé relié : la TV du salon</span><span>code 4 7 2</span></div>'
  + '<div class="chk"><i>✓</i><span>« Précédemment… » relu</span><span>écrit par le co-MJ</span></div>'
  + '<div class="chk"><i>✓</i><span>Musique : Ombres de la crypte</span><span>YouTube</span></div>'
  + '<div class="chk"><i class="{{marcChk}}">✓</i><span>Son activé chez tout le monde</span><span>{{marcChkTxt}}</span></div></div>'))
# 2 · previously
B.append(block(2, ttl('à l’écran et sur les téléphones', 'Précédemment…')
  + '<div class="edit"><span class="lbl">Écrit par le co-MJ depuis la session 2 · modifiable</span><p class="nar" style="font-size: 22px">Vous êtes descendus dans la crypte des Valombre. Les crânes regardent tous la porte nord. Borin a pris un mauvais coup.<span class="caret"></span></p></div>'
  + '<div class="sec"><span class="lbl">Ce que les joueurs savent déjà</span>'
  + f'<div class="clue">{imp("Objet", (40, 40), objet="parchemin", rarete="commune", taille=40, pips="false")}<span><b>Le registre</b> : une page a été arrachée.</span><em>session 2</em></div>'
  + f'<div class="clue">{imp("Objet", (40, 40), objet="anneau", rarete="peu", taille=40, pips="false")}<span><b>La bague du gardien</b> : gravée d’un corbeau.</span><em>session 2</em></div></div>'
  + '<div class="secret"><span class="lbl">Pour toi seul</span><span>Borin est à 15 PV sur 31. Le chef gobelin attend toujours derrière la porte est : il n’entre pas ce soir.</span></div>'))
# 3 · the scene node
B.append(block(3, ttl('nœud en cours', 'La salle de l’autel')
  + '<div style="display: grid; grid-template-columns: 300px 1fr; gap: 16px"><div class="art"><span class="tag">[ILLUSTRATION · VALIDÉE]</span></div>'
  + '<div class="sec"><span class="lbl">À lire à voix haute · montré aux joueurs</span><p class="nar" style="font-size: 21px">L’escalier s’arrête devant un autel de pierre noire. Des cendres encore tièdes. Quelqu’un est passé ici il y a moins d’une heure.</p></div></div>'
  + '<div class="secret"><span class="lbl">Pour toi seul</span><span><b>La page arrachée</b> est coincée sous l’autel : Sagesse DD 13 pour la trouver. Le culte est passé il y a une heure. Derrière la porte de droite, <b>deux gobelins</b> (rencontre prête).</span></div>'
  + '<div class="sec"><span class="lbl">Sorties</span><div class="exits"><span class="exit">Porte de droite · gobelins</span><span class="exit">Porte nord · verrouillée</span><span class="exit">Escalier · retour</span></div></div>'))
# 4 · what do you do?
B.append(block(4, ttl('les réponses arrivent', 'Que faites-vous ?')
  + '<p class="nar" style="font-size: 21px">« Devant l’autel, que faites-vous ? »</p>'
  + f'<div class="ans"><span class="pf">{imp("Sprite", (40, 52), perso="lyra", px=2, anim="repos")}</span><span class="who"><b>Lyra</b><small>Camille</small></span><p>« Je surveille l’escalier, arc en main. »</p><span class="ok">Accepté</span></div>'
  + '<sc-if value="{{notSent}}" hint-placeholder-val="{{ true }}">'
  + f'<div class="ans"><span class="pf">{imp("Sprite", (40, 52), perso="borin", px=2, anim="repos")}</span><span class="who"><b>Borin</b><small>Marc</small></span><p class="pulse" style="color: #8C8C8C">réfléchit…</p></div></sc-if>'
  + '<sc-if value="{{sent}}" hint-placeholder-val="{{ false }}"><div class="req"><div class="hd">'
  + f'<span class="pf">{imp("Sprite", (40, 52), perso="borin", px=2, anim="repos")}</span><div><span class="lbl">Borin · Marc propose</span><b>Fouiller l’autel</b><small>Chercher ce qui est caché. Borin est attentif : Sagesse +4.</small></div></div>'
  + '<div class="acts"><button type="button" class="btn">Accepter sans dé</button><button type="button" class="btn">Refuser</button><span class="lbl" style="margin-left: auto; align-self: center">ou demande un jet ci-dessous</span></div></div></sc-if>'
  + f'<div class="ans"><span class="pf">{imp("Sprite", (40, 52), perso="sef", px=2, anim="repos")}</span><span class="who"><b>Sef</b><small>Hugo</small></span><p class="pulse" style="color: #8C8C8C">réfléchit…</p></div>'))
# 5 · the roll
B.append(block(5, ttl('Borin · fouiller l’autel', 'Sagesse · DD 13')
  + '<div class="dice"><div class="dz">'
  + '<sc-if value="{{diceIdle}}" hint-placeholder-val="{{ true }}">' + imp('De', (170, 170), de='d20', valeur=20, taille=170, anim='fixe') + '<span class="pulse" style="font-size: 13px; color: #A3A3A3">Marc lance le dé sur son téléphone…</span></sc-if>'
  + '<sc-if value="{{diceRolling}}" hint-placeholder-val="{{ false }}">' + imp('De', (170, 170), de='d20', valeur=16, taille=170, anim='roule') + '<span style="font-size: 13px; color: #A3A3A3">Le dé roule…</span></sc-if>'
  + '<sc-if value="{{diceDone}}" hint-placeholder-val="{{ false }}">' + imp('De', (170, 170), de='d20', valeur=16, taille=170, anim='fixe') + '<div class="res"><span>16</span><span class="op">+ 4</span><span class="op">=</span><span>20</span></div><span class="win">RÉUSSI</span></sc-if>'
  + '</div><div class="sec"><span class="lbl">Ce que tu as prévu</span>'
  + '<div class="out {{outWin}}"><span class="lbl">Réussi · 13 ou plus</span><span>Il trouve <b>la page arrachée</b>, coincée dans une fente sous l’autel.</span></div>'
  + '<div class="out"><span class="lbl">Raté</span><span>Des cendres tièdes, et des traces de pas vers la porte de droite.</span></div>'
  + '<span style="font-size: 12px; color: #8C8C8C; line-height: 1.5">Le serveur lance le dé quand Marc touche le sien. Le résultat arrive ici, sur son téléphone et sur la TV en même temps.</span></div></div>'))
# 6 · the clue
B.append(block(6, ttl('révélé à tous', 'La page arrachée')
  + '<div style="display: grid; grid-template-columns: 220px 1fr; gap: 20px; align-items: start"><div class="flip">'
  + imp('GameCard', (200, 280), titre='La page arrachée', type='Indice', texte='Coincée sous l’autel : la page qui manquait au registre.', valeur='', stat='none', icon='parchemin', w=200)
  + '</div><div class="sec"><span class="lbl">Révélation · ce que garde la porte nord</span>'
  + f'<div class="clue">{imp("Objet", (40, 40), objet="parchemin", rarete="commune", taille=40, pips="false")}<span><b>Le registre</b></span><em>trouvé · session 2</em></div>'
  + f'<div class="clue">{imp("Objet", (40, 40), objet="anneau", rarete="peu", taille=40, pips="false")}<span><b>La bague du gardien</b></span><em>trouvé · session 2</em></div>'
  + f'<div class="clue key">{imp("Objet", (40, 40), objet="parchemin", rarete="rare", taille=40, pips="false")}<span><b>La page arrachée</b></span><em>trouvé · ce soir</em></div>'
  + '<span style="font-size: 13px; color: #D4D4D4; line-height: 1.5"><b style="color: #F2F2F2">3 indices sur 3.</b> Les joueurs ont de quoi comprendre que les Valombre gardent la porte nord. Le co-MJ ne leur dira rien : c’est à eux de faire le lien.</span></div></div>'))
# 7 · exploring the map
fx, fy = at(8, 0)
B.append(block(7, ttl('vue MJ · tu vois tout', 'La crypte')
  + plan(False, f'<div class="fogz" style="left: {fx}px; top: {fy}px; width: {6 * P}px; height: {5 * P}px"><span>Brouillard pour les joueurs</span></div>')
  + '<div class="tools"><button type="button" class="btn">Révéler une zone</button><button type="button" class="btn">Poser un pion</button><button type="button" class="btn">Ouvrir une porte</button><span class="lbl">{{exploreLbl}}</span></div>'
  + '<span class="line">{{exploreTxt}}</span>'))
# 8 · combat starts
B.append(block(8, ttl('round 1 · initiative calculée', 'Combat : gobelins de la crypte')
  + plan(True)
  + '<sc-if value="{{feed0}}" hint-placeholder-val="{{ true }}"><div class="req"><div class="hd">'
  + f'<span class="pf foe">{imp("Sprite", (40, 52), perso="gobelin", px=2, anim="repos")}</span><div><span class="lbl">Tour de Gobelin 1 · le co-MJ propose</span><b>Tirer sur Lyra</b><small>Arc court +4 contre CA 15. Lyra est la plus exposée, au pied de l’escalier.</small></div></div>'
  + '<div class="acts"><button type="button" class="btn">Changer de cible</button><button type="button" class="btn">Autre action</button></div></div></sc-if>'
  + '<sc-if value="{{feedOn}}" hint-placeholder-val="{{ false }}"><div class="out"><span class="lbl">{{feedLbl}}</span><span>{{feedTxt}}</span></div></sc-if>'))
# 9 · Borin's turn
gx, gy = at(5, 6)
B.append(block(9, ttl('round 1', 'Tour de Borin')
  + plan(True, f'<span class="tgt" style="left: {gx}px; top: {gy}px"></span><sc-if value="{{{{atkHit}}}}" hint-placeholder-val="{{{{ false }}}}"><span class="dmg" style="left: {gx + 6}px; top: {gy - 40}px">−9</span></sc-if>')
  + '<sc-if value="{{atkWait}}" hint-placeholder-val="{{ true }}"><div class="out"><span class="lbl">Marc</span><span class="pulse">{{atkWaitTxt}}</span></div></sc-if>'
  + '<sc-if value="{{atkAsking}}" hint-placeholder-val="{{ false }}"><div class="req"><div class="hd">'
  + f'<span class="pf">{imp("Sprite", (40, 52), perso="borin", px=2, anim="repos")}</span><div><span class="lbl">Borin · Marc propose une idée</span><b>« Je renverse la table sur le gobelin. »</b><small>Le co-MJ suggère : Force DD 12, ou accepter sans dé.</small></div></div>'
  + '<div class="acts"><button type="button" class="btn">Jet de Force · DD 12</button><button type="button" class="btn">Refuser</button></div></div></sc-if>'
  + '<sc-if value="{{atkDone}}" hint-placeholder-val="{{ false }}"><div class="out"><span class="lbl">Résolu par le serveur</span><span>{{atkResTxt}}</span></div></sc-if>'))
# 10 · the goblin strikes back
bxp, byp = at(4, 6)
B.append(block(10, ttl('round 1', 'Tour de Gobelin 1')
  + plan(True, f'<sc-if value="{{{{rip}}}}" hint-placeholder-val="{{{{ false }}}}"><span class="dmg" style="left: {bxp + 4}px; top: {byp - 40}px">−6</span></sc-if>')
  + '<sc-if value="{{notRip}}" hint-placeholder-val="{{ true }}"><div class="req"><div class="hd">'
  + f'<span class="pf foe">{imp("Sprite", (40, 52), perso="gobelin", px=2, anim="repos")}</span><div><span class="lbl">Tour de Gobelin 1 · le co-MJ propose</span><b>Frapper Borin au cimeterre</b><small>Il est au contact : +4 contre CA 18, 1d6 + 2 dégâts.</small></div></div>'
  + '<div class="acts"><button type="button" class="btn">Changer de cible</button><button type="button" class="btn">Fuir</button></div></div></sc-if>'
  + '<sc-if value="{{rip}}" hint-placeholder-val="{{ false }}"><div class="out"><span class="lbl">Résolu par le serveur</span><span>19 + 4 = 23 contre CA 18 : touché, <b style="color: #FF4D5E">6 dégâts</b>. Borin passe à 9 PV sur 31.</span></div>'
  + f'<div class="loot" style="padding: 10px 14px; gap: 14px">{imp("Objet", (48, 48), objet="hache", rarete="leg", taille=48, pips="false")}<span style="font-size: 13px; line-height: 1.45; flex: 1"><span class="lbl" style="display: block">Butin prévu pour ce combat</span><b>Hache des Valombre</b> · légendaire, pour Borin</span><button type="button" class="btn">Changer</button></div></sc-if>'))
# 11 · loot
B.append(block(11, ttl('validé · sur la TV et dans le sac', 'Butin')
  + f'<div class="loot">{imp("Objet", (140, 140), objet="hache", rarete="leg", taille=140)}<div class="sec" style="flex: 1"><span class="ttl" style="font-size: 26px; color: #F3CC63">Hache des Valombre</span><span class="lbl" style="color: #F3CC63">Légendaire · pour Borin</span><span style="font-size: 13px; color: #D4D4D4">1d12 + 3 dégâts, +2 pour toucher.</span><span style="font-size: 13px"><b>{{{{lootGm}}}}</b></span></div></div>'
  + '<div class="sec"><span class="lbl">Expérience</span>'
  + ''.join(f'<div class="clue"><span class="pf">{imp("Sprite", (20, 26), perso=p, px=1, anim="repos")}</span><span style="width: 60px"><b>{n}</b></span>{imp("Cases", (200, 16), stat="ini", valeur=v, max=10, h=10, largeur=200)}<em>{v} / 10</em></div>' for p, n, v in [('lyra', 'Lyra', 7), ('borin', 'Borin', 8), ('sef', 'Sef', 6)])
  + '</div>'))
# 12 · end of session
B.append(block(12, ttl('écrits par le co-MJ · à relire', 'Fin de la session')
  + '<div class="two"><div class="edit"><span class="lbl">Pour toi · récapitulatif MJ</span><ul style="margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 6px; font-size: 13px; line-height: 1.45; color: #D4D4D4"><li>Indices de la porte nord : 3 sur 3. Ils peuvent l’ouvrir jeudi.</li><li>Borin a 9 PV, la hache légendaire est équipée.</li><li>Le chef gobelin, derrière la porte est, n’a pas été vu.</li><li>Le culte des cendres n’a pas avancé : proposé 3 → 4.</li></ul></div>'
  + '<div class="edit"><span class="lbl">Pour les joueurs · « Précédemment… »</span><p class="nar" style="font-size: 19px">Sous l’autel, Borin a trouvé la page arrachée. Les gobelins de la crypte sont tombés. La porte nord est ouverte : quelque chose chante derrière.<span class="caret"></span></p></div></div>'
  + '<div class="clue" style="padding: 12px 14px"><span class="lbl" style="width: 150px">Prochaine session</span><b class="ttl" style="font-size: 18px">Jeudi 10 octobre · 20 h 30</b><em>envoyé aux joueurs avec le récap</em></div>'))

LEFT = '''<div class="col">
<section class="panel"><div class="ph"><span class="lbl">La soirée</span><span class="lbl">{{clock}}</span></div><div class="track"><sc-for list="{{steps}}" as="s" hint-placeholder-count="7"><div class="st {{s.cls}}"><i></i><span>{{s.n}}</span><small>{{s.h}}</small></div></sc-for></div></section>
<section class="panel" style="flex: 1; min-height: 0"><div class="ph"><span class="lbl">{{tblLbl}}</span><span class="lbl">{{tblSub}}</span></div><div class="rows"><sc-for list="{{rows}}" as="r" hint-placeholder-count="3"><div class="row {{r.cls}}"><sc-if value="{{r.isIni}}" hint-placeholder-val="{{ false }}"><span class="ini">{{r.ini}}</span></sc-if><span class="pf {{r.pf}}"><dc-import name="Sprite" perso="{{r.p}}" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><div style="flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px"><div class="nm"><span>{{r.n}}</span><small>{{r.w}}</small></div><sc-if value="{{r.isHp}}" hint-placeholder-val="{{ true }}"><div style="display: flex; align-items: center; gap: 8px"><dc-import name="Coeurs" pv="{{r.pv}}" max="{{r.max}}" coeurs="8" px="1" anim="false" hint-size="76px,8px"></dc-import><span class="hp">{{r.pv}}/{{r.max}}</span></div></sc-if><span class="sub">{{r.sub}}</span></div></div></sc-for></div></section>
<section class="panel"><div class="ph"><span class="lbl">Menace</span><span class="lbl">pour toi seul</span></div><div class="front"><dc-import name="Horloge" parts="6" rempli="3" taille="56" texte="false" hint-size="56px,56px"></dc-import><p><b>Le culte des cendres</b>Avance d’une part si le groupe s’attarde. Prochaine part : la porte nord se referme.</p></div></section>
</div>'''

CENTER = '<div class="col"><div class="bn {{bnCls}}"><span>{{bnTxt}}</span></div><section class="panel stage">' + ''.join(B) + '</section>' + go() + '</div>'

RIGHT = '''<div class="col">
<section class="panel"><div class="ph"><span class="ai"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"></path></svg><span class="lbl" style="color: #F2F2F2">Co-MJ</span></span><span class="lbl">{{coKind}}</span></div>
<div class="co"><div class="edit" style="padding: 12px 14px"><span class="lbl">Brouillon · rien ne part sans toi</span><p class="nar" style="font-size: 18px">{{coText}}</p></div>
<sc-for list="{{sugg}}" as="g" hint-placeholder-count="2"><div class="sugg"><code>{{g.c}}</code><span>{{g.t}}</span><button type="button" class="btn">Appliquer</button></div></sc-for>
<div style="display: flex; gap: 8px"><button type="button" class="btn">Modifier</button><button type="button" class="btn">Relancer</button><button type="button" class="btn" style="margin-left: auto">Dire autre chose…</button></div></div></section>
<section class="panel" style="flex: 1; min-height: 0; display: flex; flex-direction: column"><div class="ph"><span class="lbl">Journal de la soirée</span><span class="lbl">tout, même le caché</span></div><div class="jr"><sc-for list="{{events}}" as="e" hint-placeholder-count="5"><div class="ev {{e.cls}}"><time>{{e.h}}</time><span>{{e.t}}</span></div></sc-for></div></section>
</div>'''

TOP = '''<header class="top"><span class="ttl" style="font-size: 13px; letter-spacing: .3em; color: #F2F2F2">PROMPTUS</span><span class="sep"></span><span class="ttl" style="font-size: 15px; color: #F2F2F2">Session 3 · La crypte des Valombre</span>
<span style="display: flex; align-items: center; gap: 8px"><span class="pres"></span>3 joueurs</span>
<span class="tvnow"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="4" width="18" height="12" rx="2"></rect><path d="M8 20h8"></path></svg>TV : <b>{{tvNow}}</b></span>
<span style="flex: 1"></span>
<span class="yt"><span class="thumb"><span class="eq"><i></i><i></i><i></i></span></span><span style="display: flex; flex-direction: column; gap: 1px"><b style="color: #F2F2F2; font-size: 12px; font-weight: 600">{{track}}</b><span style="font-size: 10px; color: #8C8C8C">YouTube · 3 sur 3 synchronisés</span></span></span>
<span>IA 0,42 / 10 $</span>
<button type="button" class="btn">Terminer la session</button></header>'''

lap = ('<div class="lap">' + TOP + '<div class="cols">' + LEFT + CENTER + RIGHT + '</div>'
       + '<sc-if value="{{ended}}" hint-placeholder-val="{{ false }}"><div class="endc"><span class="lbl">Session 3 enregistrée</span><span class="ttl" style="font-size: 40px">Bonne nuit, Romain.</span><p class="nar" style="font-size: 22px">Le récapitulatif est parti aux joueurs. La chronique de la campagne a une page de plus.</p>'
       + '<button type="button" class="go" style="flex: none; width: 280px" onClick="{{restart}}"><span class="in"><span style="flex: 1"><span class="t">Rejouer la soirée</span></span><span class="chev">›</span></span></button></div></sc-if></div>')

SIDE = '''<div class="side">
<div style="display: flex; justify-content: space-between; align-items: baseline"><span class="lbl">La TV du salon</span><span class="lbl">suit l’écran du MJ</span></div>
<div class="tvf"><div style="width: 1920px; height: 1080px; transform: scale(.3333); transform-origin: 0 0"><dc-import name="Ecran-TV" moment="{{tvM}}" hp="{{tvHp}}" deg="{{tvDeg}}" dice="{{tvDice}}" atk="{{tvAtk}}" card="{{tvCard}}" feed="{{tvFeed}}" choice="0" sent="{{tvSent}}" moved="{{tvMoved}}" bx="{{bx}}" by="{{by}}" equipped="{{tvEquipped}}" hint-size="1920px,1080px"></dc-import></div></div>
<span class="lbl">Sur le téléphone de Marc</span><div class="seen">{{marcSees}}</div>
<span class="lbl">Dans la tête de Romain, le MJ</span><div class="think">{{thought}}</div>
<sc-if value="{{is9}}" hint-placeholder-val="{{ false }}"><span class="lbl">Ce que Marc joue à son tour</span><div class="pick"><sc-for list="{{plays}}" as="p" hint-placeholder-count="2"><button type="button" class="{{p.cls}}" onClick="{{p.pick}}">{{p.n}}</button></sc-for></div></sc-if>
<span class="lbl">La soirée · moment {{m}} sur 12</span><div class="prog"><sc-for list="{{dots}}" as="d" hint-placeholder-count="12"><button type="button" class="{{d.cls}}" onClick="{{d.pick}}">{{d.n}}</button></sc-for></div>
<div class="ctl"><button type="button" onClick="{{restart}}">Recommencer</button></div>
</div>'''

JS = r'''
class Component extends DCLogic {
  constructor(props) {
    super(props);
    this.state = this.fresh();
    const pm = Number(props && props.moment);
    this.timers = [];
    if (pm >= 1 && pm <= 12) Object.assign(this.state, this.settled(pm));
  }
  fresh() {
    return { m: 1, sound: false, sent: false, dice: 'idle', moved: false, feed: 0, play: 'hache', atk: 'wait', rip: false, equipped: '', ended: false };
  }
  // A frozen moment (storyboard) shows the state once the players have acted.
  settled(m) {
    return { m, sound: true, sent: m === 4, dice: m === 5 ? 'done' : 'idle', moved: m === 7, feed: 0, atk: m === 9 ? 'done' : 'wait', rip: m === 10, equipped: m === 11 ? 'equip' : '' };
  }
  later(fn, ms) { this.timers.push(setTimeout(fn, ms)); }
  clear() { this.timers.forEach((t) => clearTimeout(t)); this.timers = []; }
  go(m) {
    if (m > 12) return;
    this.clear();
    const patch = { m, ended: false };
    if (m <= 4) patch.sent = false;
    if (m <= 5) patch.dice = 'idle';
    if (m <= 7) patch.moved = false;
    if (m <= 8) patch.feed = 0;
    if (m <= 9) patch.atk = 'wait';
    if (m <= 10) patch.rip = false;
    if (m <= 11) patch.equipped = '';
    if (m === 1) patch.sound = false;
    this.setState(patch);
    if (m === 1) this.later(() => this.setState({ sound: true }), 2500);
    if (m === 4) this.later(() => this.setState({ sent: true }), 3000);
    if (m === 5) { this.later(() => this.setState({ dice: 'rolling' }), 2200); this.later(() => this.setState({ dice: 'done' }), 3800); }
    if (m === 7) this.later(() => this.setState({ moved: true }), 2500);
    if (m === 9) this.turn();
    if (m === 11) this.later(() => this.setState({ equipped: 'equip' }), 3000);
  }
  turn() {
    this.setState({ atk: 'wait' });
    if (this.state.play === 'idee') this.later(() => this.setState({ atk: 'asking' }), 2600);
    else { this.later(() => this.setState({ atk: 'rolling' }), 2600); this.later(() => this.setState({ atk: 'done' }), 4100); }
  }
  renderVals() {
    const s = this.state;
    const m = s.m;
    // Nothing has started a timer yet when the board opens on the lobby: Marc still has to arrive.
    if (!this.kicked) { this.kicked = true; if (m === 1 && !s.sound) this.later(() => this.setState({ sound: true }), 2500); }
    const idee = s.play === 'idee';
    const hp = m > 10 || (m === 10 && s.rip) ? 9 : 15;
    const steps = [['Salon', [1], '20:30'], ['Précédemment…', [2], '20:31'], ['La salle de l’autel', [3, 4, 5, 6], '20:34'], ['Exploration', [7], '20:42'], ['Combat : gobelins', [8, 9, 10], '20:43'], ['Butin', [11], '20:52'], ['Fin de la session', [12], '21:40']];
    const combat = m >= 8 && m <= 10;
    const g1 = m === 9 && s.atk === 'done' && !idee || m === 10 ? 7 : 16;
    const g2 = m === 8 && s.feed < 2 ? 16 : 12;
    const turnOf = m === 8 ? (s.feed < 2 ? 0 : 1) : m === 9 ? 2 : m === 10 ? 0 : -1;
    const party = [
      { p: 'lyra', n: 'Lyra', w: 'Camille', pv: 22, max: 22 },
      { p: 'borin', n: 'Borin', w: 'Marc', pv: hp, max: 31 },
      { p: 'sef', n: 'Sef', w: 'Hugo', pv: 14, max: 16 }
    ];
    const subs = {
      1: ['prête · son activé', s.sound ? 'prêt · son activé' : 'arrive… son coupé', 'prêt · son activé'],
      2: ['écoute', 'écoute', 'écoute'], 3: ['écoute', 'écoute', 'écoute'],
      4: ['a répondu', s.sent ? 'a proposé · à toi' : 'réfléchit…', 'réfléchit…'],
      5: ['regarde', s.dice === 'done' ? 'a fait 20' : 'lance le dé…', 'regarde'],
      6: ['regarde', 'a trouvé l’indice', 'regarde'],
      7: ['suit Borin', s.moved ? 'a avancé' : 'choisit sa case…', 'reste en arrière'],
      11: ['regarde', s.equipped ? 'a équipé la hache' : 'regarde son butin…', 'regarde'],
      12: ['a dit à jeudi', 'a dit à jeudi', 'a dit à jeudi']
    }[m] || ['', '', ''];
    const rows = combat
      ? [
          { p: 'gobelin', n: 'Gobelin 1', w: 'CA 13', pv: g1, max: 16, ini: 17, foe: true },
          { p: 'sef', n: 'Sef', w: 'Hugo', pv: 14, max: 16, ini: 15 },
          { p: 'borin', n: 'Borin', w: 'Marc', pv: hp, max: 31, ini: 12 },
          { p: 'lyra', n: 'Lyra', w: 'Camille', pv: 22, max: 22, ini: 9 },
          { p: 'gobelin', n: 'Gobelin 2', w: 'CA 13', pv: g2, max: 16, ini: 6, foe: true }
        ].map((r, i) => ({ ...r, cls: i === turnOf ? 'now' : '', pf: r.foe ? 'foe' : '', isIni: true, isHp: true, sub: r.foe ? 'PV cachés aux joueurs' : '' }))
      : party.map((r, i) => ({ ...r, cls: '', pf: '', isIni: false, isHp: true, ini: '', sub: subs[i] }));
    const atkTv = { wait: 'idle', rolling: 'rolling', asking: 'asking', done: 'done' };
    const bn = {
      1: s.sound ? ['you', 'À toi : tout le monde est là, lance la session'] : ['wait', 'Les joueurs arrivent…'],
      2: ['live', '« Précédemment… » passe à l’écran'],
      3: ['you', 'Lis la scène, puis demande ce qu’ils font'],
      4: s.sent ? ['you', 'À toi : Marc propose de fouiller l’autel'] : ['wait', 'Les joueurs réfléchissent…'],
      5: s.dice === 'done' ? ['you', 'Réussi : révèle l’indice'] : ['wait', 'Marc lance le dé…'],
      6: ['live', 'L’indice est à l’écran'],
      7: ['live', s.moved ? 'Borin s’approche de la porte de droite' : 'Les joueurs explorent la carte'],
      8: s.feed === 0 ? ['turn', 'Tour de Gobelin 1 : à toi de le jouer'] : s.feed === 1 ? ['wait', 'Tour de Sef : Hugo joue…'] : ['live', 'Sef a joué · au tour de Borin'],
      9: s.atk === 'asking' ? ['you', 'À toi : Marc propose une idée'] : s.atk === 'done' ? ['live', 'Borin a joué'] : ['wait', 'Tour de Borin : Marc joue…'],
      10: s.rip ? ['live', 'Le gobelin a touché Borin'] : ['turn', 'Tour de Gobelin 1 : à toi de le jouer'],
      11: ['live', 'Le butin tombe sur la TV'],
      12: ['you', 'Relis les récapitulatifs, puis publie']
    }[m];
    const main = {
      1: ['Lancer la session', 'La TV et les téléphones passent à « Précédemment… »', s.sound, () => this.go(2)],
      2: ['Passer à la scène', 'La salle de l’autel', true, () => this.go(3)],
      3: ['Demander : que faites-vous ?', 'La question s’affiche sur les téléphones et la TV', true, () => this.go(4)],
      4: s.sent ? ['Demander un jet · Sagesse DD 13', 'Marc lance le dé sur son téléphone', true, () => this.go(5)] : ['En attente des propositions', 'Les cartes arrivent ici', false, null],
      5: s.dice === 'done' ? ['Révéler l’indice : la page arrachée', 'Il s’ajoute au journal de chaque joueur', true, () => this.go(6)] : ['En attente du dé', 'Marc doit faire 13 ou plus', false, null],
      6: ['Passer à l’exploration', 'La carte de la crypte s’ouvre partout', true, () => this.go(7)],
      7: ['Lancer la rencontre : gobelins de la crypte', 'Initiative calculée, carte de combat partout', true, () => this.go(8)],
      8: s.feed === 0 ? ['Jouer ce tour', 'Gobelin 1 tire sur Lyra', true, () => { this.setState({ feed: 1 }); this.later(() => this.setState({ feed: 2 }), 2400); }]
        : s.feed === 1 ? ['Hugo joue Sef…', 'Son tour se règle sur son téléphone', false, null] : ['Tour suivant : Borin', 'Marc joue sur son téléphone', true, () => this.go(9)],
      9: s.atk === 'asking' ? ['Accepter : le gobelin tombe', 'Il est à terre jusqu’à son prochain tour', true, () => this.setState({ atk: 'done' })]
        : s.atk === 'done' ? ['Tour suivant : Gobelin 1', 'C’est à toi de le jouer', true, () => this.go(10)] : ['Marc joue son tour…', 'Le serveur vérifie portée et déplacement', false, null],
      10: s.rip ? ['Finir le combat et donner le butin', 'Lyra et Sef achèvent les gobelins', true, () => this.go(11)] : ['Jouer ce tour', 'Gobelin 1 frappe Borin', true, () => this.setState({ rip: true })],
      11: ['Terminer la session', 'Le co-MJ écrit les récapitulatifs', true, () => this.go(12)],
      12: ['Publier et clore la session', 'Récap aux joueurs, chronique enregistrée', true, () => { this.clear(); this.setState({ ended: true }); }]
    }[m];
    const co = {
      1: ['prêt pour ce soir', 'Vous êtes descendus dans la crypte des Valombre. Les crânes regardent tous la porte nord…', [['scene', 'Ouvrir sur : la salle de l’autel'], ['music', 'Ombres de la crypte']]],
      2: ['et ensuite ?', 'Lis la scène de l’autel, puis laisse-les fouiller : la page arrachée y est cachée.', []],
      3: ['décrire', 'Une odeur de cire froide. Sur l’autel, une coupe renversée, et des traces de doigts dans la cendre.', []],
      4: ['conséquences', 'Fouiller l’autel : Sagesse DD 13. Réussi, la page arrachée. Raté, des cendres tièdes et des traces vers la porte de droite.', [['ask_roll', 'Sagesse DD 13 pour Borin']]],
      5: ['décrire', 'Sous l’autel, coincée dans une fente… une page de parchemin. La page arrachée du registre !', [['reveal_clue', 'La page arrachée']]],
      6: ['et ensuite ?', 'Le bruit a porté. Derrière la porte de droite, des griffes raclent la pierre.', [['advance_scene', 'Exploration de la crypte']]],
      7: ['conséquences', 'Si Borin passe la porte de droite, les deux gobelins attaquent par surprise.', [['start_encounter', 'Gobelins de la crypte']]],
      8: ['adversaires', 'Le gobelin glapit et décoche une flèche vers la silhouette au pied de l’escalier.', []],
      9: idee ? ['conséquences', 'La table bascule dans un fracas. Le gobelin s’étale, son arc sous lui.', [['add_state', 'Gobelin 1 : à terre, 1 tour']]] : ['faire parler', '« Le maire paie pour que personne ne descende ! » glapit le gobelin.', []],
      10: ['adversaires', 'Il se relève d’un bond et vise le défaut de l’armure.', []],
      11: ['récompense', 'La hache était prévue pour ce combat. Borin en a besoin : 9 PV, la porte nord jeudi.', [['give_item', 'Hache des Valombre → Borin']]],
      12: ['récapitulatif', 'Les deux récapitulatifs sont prêts au centre. Rien ne part avant que tu publies.', [['advance_front', 'Culte des cendres : 3 → 4'], ['next_scene', 'La porte nord']]]
    }[m];
    const evs = [
      [1, '20:30', 'Camille, Marc et Hugo sont connectés. TV reliée.', ''],
      [2, '20:31', '« Précédemment… » diffusé.', ''],
      [3, '20:34', 'Scène montrée : la salle de l’autel.', ''],
      [4, '20:36', 'Lyra surveille l’escalier (accepté).', ''],
      [5, '20:38', 'Borin fouille l’autel : Sagesse 16 + 4 = 20, réussi.', ''],
      [6, '20:39', 'Indice révélé : la page arrachée (3 sur 3).', 'key'],
      [7, '20:42', 'Borin avance vers la porte de droite.', ''],
      [8, '20:43', 'Rencontre : 2 gobelins. Initiative G1 17, Sef 15, Borin 12, Lyra 9, G2 6.', 'key'],
      [8.1, '20:43', 'G1 tire sur Lyra : 9 contre CA 15, raté.', ''],
      [8.2, '20:44', 'Sef lance une dague sur G2 : 4 dégâts (12/16 PV).', 'hid'],
      [9, '20:44', idee ? 'Borin renverse la table : G1 à terre (idée acceptée).' : 'Borin frappe G1 : 22 contre CA 13, 9 dégâts (7/16 PV).', idee ? '' : 'hid'],
      [10, '20:45', 'G1 frappe Borin : 23 contre CA 18, 6 dégâts (9/31 PV).', ''],
      [10.5, '20:51', 'Les gobelins tombent. Fin du combat.', 'key'],
      [11, '20:52', 'Butin validé : Hache des Valombre → Borin.', 'key'],
      [12, '21:40', 'Fin de la session. Prochaine : jeudi 20 h 30.', 'key']
    ].filter(([k]) => {
      if (k === 4) return m > 4 || (m === 4 && s.sent);
      if (k === 5) return m > 5 || (m === 5 && s.dice === 'done');
      if (k === 7) return m > 7 || (m === 7 && s.moved);
      if (k === 8.1) return m > 8 || (m === 8 && s.feed >= 1);
      if (k === 8.2) return m > 8 || (m === 8 && s.feed >= 2);
      if (k === 9) return m > 9 || (m === 9 && s.atk === 'done');
      if (k === 10) return m > 10 || (m === 10 && s.rip);
      if (k === 10.5) return m > 10;
      return k <= m;
    }).reverse().map(([, h, t, cls]) => ({ h, t: cls === 'hid' ? t + ' · caché aux joueurs' : t, cls }));
    const thoughts = {
      1: s.sound ? 'Tout le monde est là, le son marche chez Marc. Je lance.' : 'Marc arrive. J’attends qu’il active le son.',
      2: 'Le co-MJ l’a écrit, je l’ai relu ce matin. Je le laisse passer et je lis en même temps.',
      3: 'Je lis le texte à voix haute sur Discord. La page est sous l’autel : je ne dis rien.',
      4: s.sent ? 'Marc veut fouiller. C’est exactement là que se trouve la page : un jet suffit.' : 'J’attends leurs réponses. Lyra a déjà répondu.',
      5: s.dice === 'done' ? '20 ! Je révèle la page.' : 'Il lui faut 13. Je n’ai rien à faire, le serveur lance.',
      6: 'Trois indices sur trois. Je ne leur souffle rien, ils vont faire le lien.',
      7: s.moved ? 'Borin est devant la porte. Les gobelins attendent derrière : c’est le moment.' : 'Ils voient la salle, pas le nord-est. Je les laisse avancer.',
      8: s.feed === 0 ? 'Le co-MJ me propose de tirer sur Lyra. Ça me va, je joue.' : s.feed === 1 ? 'Raté. Hugo joue Sef pendant ce temps.' : 'Au tour de Borin. Marc a ses cartes en main.',
      9: s.atk === 'asking' ? 'Renverser la table ? J’adore. J’accepte sans dé.' : s.atk === 'done' ? (idee ? 'Le gobelin est à terre. À son tour, il se relève.' : 'Touché, 9 dégâts. Il lui reste 7 PV, les joueurs ne le savent pas.') : 'Marc choisit. Le serveur vérifiera la portée.',
      10: s.rip ? 'Borin est à 9 PV. Je finis le combat et je donne la hache prévue.' : 'Le gobelin riposte sur Borin. Je valide la proposition.',
      11: s.equipped ? 'Marc l’a équipée tout de suite. On s’arrête là pour ce soir.' : 'La hache tombe sur la TV. Marc décide quoi en faire.',
      12: 'Je relis les deux récaps, je corrige un mot, je publie.'
    }[m];
    const sees = {
      1: s.sound ? 'Il a activé le son. « C’est parti, le MJ lance la partie… »' : 'Il arrive dans le salon et voit « Activer le son ».',
      2: '« Précédemment… », les indices déjà trouvés. Rien à faire : il écoute.',
      3: 'La scène, son illustration, le texte qui s’écrit. Rien à faire pour l’instant.',
      4: s.sent ? 'Il a choisi « Fouiller l’autel » et attend : « Le MJ lit ta proposition… »' : 'Quatre choix et « Autre chose… ». Il hésite.',
      5: s.dice === 'done' ? '16 + 4 = 20, RÉUSSI.' : '« À toi : lance le dé. » Il touche le d20.',
      6: 'La carte d’indice se retourne. Elle est dans son journal.',
      7: s.moved ? 'Sa case choisie, près de la porte de droite.' : 'La carte et ses cases vertes. Le nord-est est dans le brouillard.',
      8: 'L’ordre du tour, le gobelin d’abord. Après Sef, ce sera lui.',
      9: s.atk === 'asking' ? 'Il a écrit son idée : « Le MJ lit ton idée… »' : s.atk === 'done' ? (idee ? 'Son idée est acceptée : le gobelin est à terre.' : 'Touché ! La hache fait 9 dégâts.') : 'Ses cartes en éventail, le gros bouton sous le pouce.',
      10: s.rip ? '« Le gobelin te touche : −6 PV. » Un conseil : son second souffle.' : 'C’est au gobelin. Il regarde.',
      11: s.equipped ? 'Hache des Valombre : équipée.' : '« Butin ! » L’équiper, ou la garder dans le sac.',
      12: 'Ce soir, en trois lignes. Prochaine session : jeudi.'
    }[m];
    const tvm = m === 10 && !s.rip ? 9 : m;
    const v = {
      m, bx: m === 7 && s.moved ? 6 : 4, by: 6,
      clock: { 1: '20:30', 2: '20:31', 3: '20:34', 4: '20:36', 5: '20:38', 6: '20:39', 7: '20:42', 8: '20:43', 9: '20:44', 10: '20:45', 11: '20:52', 12: '21:40' }[m],
      steps: steps.map(([n, ms, h]) => ({ n, h, cls: ms.includes(m) ? 'cur' : ms[0] < m ? 'done' : '' })),
      tblLbl: combat ? 'Ordre du tour' : 'La table', tblSub: combat ? 'round 1' : '3 sur 3', rows,
      bnCls: bn[0], bnTxt: bn[1],
      mainLbl: main[0], mainSub: main[1], mainCls: main[2] ? '' : 'off', mainAct: () => { if (main[3]) main[3](); },
      coKind: co[0], coText: co[1], sugg: co[2].map(([c, t]) => ({ c, t })), events: evs,
      tvNow: { 1: 'salon', 2: 'Précédemment…', 3: 'la scène', 4: 'que faites-vous ?', 5: 'le dé de Borin', 6: 'l’indice', 7: 'la carte', 8: 'le combat', 9: 'le combat', 10: 'le combat', 11: 'le butin', 12: 'le récap' }[m],
      track: m >= 8 && m <= 10 ? 'Tambours des gobelins' : 'Ombres de la crypte',
      marcSound: s.sound ? 'son activé' : 'son coupé…', marcChk: s.sound ? '' : 'pend', marcChkTxt: s.sound ? '3 sur 3' : 'Marc : pas encore',
      notSent: !s.sent, sent: s.sent,
      diceIdle: s.dice === 'idle', diceRolling: s.dice === 'rolling', diceDone: s.dice === 'done', outWin: s.dice === 'done' ? 'on' : '',
      exploreLbl: s.moved ? 'Borin a bougé' : 'les joueurs bougent',
      exploreTxt: s.moved ? 'Borin s’arrête devant la porte de droite. Derrière, Gobelin 1 et Gobelin 2 attendent : eux ne le voient pas encore.' : 'Les joueurs voient la salle de l’autel. Le nord-est reste dans le brouillard : un gobelin y guette, et un coffre y est caché.',
      feed0: s.feed === 0, feedOn: s.feed > 0,
      feedLbl: s.feed === 1 ? 'Gobelin 1 · résolu par le serveur' : 'Sef · joué par Hugo',
      feedTxt: s.feed === 1 ? '9 contre CA 15 : raté. La flèche se plante dans la marche, à côté de Lyra.' : 'Dague lancée sur Gobelin 2 : touché, 4 dégâts. Il lui reste 12 PV sur 16.',
      atkWait: s.atk === 'wait' || s.atk === 'rolling', atkWaitTxt: s.atk === 'rolling' ? 'Frappe à la hache… le dé roule.' : 'choisit son action sur son téléphone…',
      atkAsking: s.atk === 'asking', atkDone: s.atk === 'done', atkHit: m === 9 && s.atk === 'done' && !idee,
      atkResTxt: idee ? 'Idée acceptée : Gobelin 1 est à terre jusqu’à son prochain tour.' : 'Hache d’armes : 17 + 5 = 22 contre CA 13, touché. 9 dégâts : Gobelin 1 passe à 7 PV sur 16.',
      rip: s.rip, notRip: !s.rip,
      lootGm: s.equipped ? 'Marc l’a équipée.' : 'Marc choisit : l’équiper ou la garder…',
      ended: s.ended,
      marcSees: sees, thought: thoughts,
      plays: [['hache', 'Sa hache (le plus courant)'], ['idee', 'Une idée à lui (« Autre… »)']].map(([id, n]) => ({ n, cls: s.play === id ? 'on' : '', pick: () => { this.clear(); this.setState({ play: id }); this.later(() => this.turn(), 50); } })),
      dots: Array.from({ length: 12 }, (_, i) => ({ n: i + 1, cls: i + 1 === m ? 'cur' : i + 1 < m ? 'done' : '', pick: () => this.go(i + 1) })),
      restart: () => { this.clear(); this.setState(this.fresh()); this.go(1); },
      tvM: tvm, tvHp: hp, tvDeg: m === 10 && s.rip ? 6 : 0, tvDice: s.dice, tvAtk: m >= 10 ? 'done' : atkTv[s.atk], tvCard: idee ? 3 : 0,
      tvFeed: s.feed, tvSent: String(s.sent), tvMoved: String(s.moved || m > 7), tvEquipped: s.equipped
    };
    for (let i = 1; i <= 12; i++) v['is' + i] = m === i;
    const seul = String(this.props.seul ?? false) === 'true';
    v.full = !seul;
    v.rootStyle = seul ? 'width: 1440px; height: 900px; padding: 0' : 'width: 2240px; height: 1020px; padding: 60px 56px';
    return v;
  }
}
'''

html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Mener la soirée de Marc</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700;800&amp;family=Chakra+Petch:wght@400;500;600;700&amp;family=Cormorant+Garamond:ital,wght@1,500;1,600&amp;display=swap">
<style>{css}</style>
</helmet>
<div class="pm" style="{{{{rootStyle}}}}; position: relative; overflow: hidden; display: flex; gap: 48px; box-sizing: border-box; align-items: flex-start">
{lap}
<sc-if value="{{{{full}}}}" hint-placeholder-val="{{{{ true }}}}">{SIDE}</sc-if>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"moment":{{"editor":"int","default":1,"min":1,"max":12}},"seul":{{"editor":"boolean","default":false}},"$preview":{{"width":2240,"height":1020}}}}'>{JS}</script>
</body>
</html>
'''
out = 'docs/design/canvas/Mener-MJ.dc.html'
open(out, 'w').write(html)
d = json.load(open('docs/design/canvas/canvas.json'))
d['boards'].setdefault('Mener-MJ.dc.html', {"x": 3040, "y": 1880, "w": 2240, "h": 1020, "title": "Mener · la soirée de Marc côté MJ", "is_interactive": True})  # keep a layout set on the canvas
if 'Mener-MJ.dc.html' not in d['order']: d['order'].append('Mener-MJ.dc.html')
json.dump(d, open('docs/design/canvas/canvas.json', 'w'), ensure_ascii=False, indent=2)

import sys as _s; _s.path.insert(0, 'docs/design')
from scope import scope, check
_out, _ren, _dyn = scope(open(out).read(), 'mm-', keep={'on', 'off', 'done', 'cur', 'now', 'foe', 'you', 'wait', 'live', 'turn', 'key', 'hid', 'pend'})
open(out, 'w').write(_out)
print(len(_out), 'unsafe:', check(_out, 'mm-', _dyn))
