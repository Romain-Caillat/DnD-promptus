"""Generate the playable GM prep board: Romain builds "Les Cendres de Valombre" (Preparer-MJ.dc.html).

Ten moments on the GM laptop, from the campaign list to a playable campaign: universe, rules,
pitch, live generation, the workshop with the co-GM, coherence alerts, sheets, maps and media,
validation. It replaces the earlier "Construction d'histoire" board, whose workshop is moment 6.
Run from the repository root.
"""
import json

css = r'''
body{margin:0;background:#0A0A0A}
.pm{font-family:'Chakra Petch',system-ui,sans-serif;color:#F2F2F2;background:radial-gradient(120% 60% at 50% 0%,#1C1C1C 0,#0C0C0C 55%,#050505 100%);-webkit-font-smoothing:antialiased}
.ttl{font-family:'Cinzel',Georgia,serif;font-weight:800;letter-spacing:.02em}
.nar{margin:0;font-family:'Cormorant Garamond',Georgia,serif;font-style:italic;font-weight:500;line-height:1.35;color:#E5E5E5}
.lbl{font-size:10px;font-weight:600;letter-spacing:.2em;text-transform:uppercase;color:#8C8C8C}
.lap{position:relative;width:1440px;height:900px;flex:none;overflow:hidden;box-sizing:border-box;display:grid;grid-template-rows:52px 46px 1fr 78px;background:radial-gradient(90% 70% at 50% 0%,#1A1A1A 0,#0C0C0C 60%,#050505 100%);border-radius:14px;box-shadow:0 0 0 2px #2C2C2C,0 30px 60px rgba(0,0,0,.7)}
.top{display:flex;align-items:center;gap:18px;padding:0 18px;border-bottom:1px solid #1F1F1F;font-size:12px;color:#A3A3A3;white-space:nowrap}
.top .sep{width:1px;height:20px;background:#2A2A2A;flex:none}
.chipst{font-size:11px;font-weight:600;color:#A3A3A3;border:1px solid #333;border-radius:6px;padding:3px 8px}
.budget{display:flex;flex-direction:column;gap:5px;width:170px}
.budget .bar{height:6px;background:#262626;border-radius:1px;overflow:hidden}
.budget .bar > i{display:block;height:100%;background:#F2F2F2}
.steps{display:flex;align-items:center;gap:0;padding:0 18px;border-bottom:1px solid #1F1F1F;background:#0B0B0B}
.sp{display:flex;align-items:center;gap:8px;font-size:11px;font-weight:600;letter-spacing:.08em;text-transform:uppercase;color:#5A5A5A;white-space:nowrap}
.sp > i{width:9px;height:9px;transform:rotate(45deg);border:1.5px solid currentColor;box-sizing:border-box;flex:none}
.sp.done{color:#A3A3A3}
.sp.done > i{background:#A3A3A3}
.sp.cur{color:#F2F2F2}
.sp.cur > i{background:#F2F2F2}
.rail{flex:1;height:1px;margin:0 12px;background:repeating-linear-gradient(90deg,#3A3A3A 0 5px,transparent 5px 10px);min-width:12px}
.body{flex:1;min-height:0;overflow:hidden;padding:16px 18px 0;display:flex;flex-direction:column;gap:14px}
.foot{display:flex;align-items:center;gap:14px;padding:0 18px 14px}
.bn{flex:1;display:flex;align-items:center;gap:10px;height:48px;padding:0 16px;border-radius:12px;font-weight:700;font-size:15px;box-sizing:border-box}
.bn.you{background:#161616;border:1px solid #2C2C2C;color:#F2F2F2}
.bn.wait{background:#0A0A0A;border:1.5px dashed #5E5E5E;color:#D4D4D4}
.bn.warn{background:#0A0A0A;border:1.5px solid #FF9F1C;color:#F2F2F2}
.go{font:inherit;width:480px;flex:none;display:block;box-sizing:border-box;border:0;padding:6px;border-radius:12px;background:#EDEDED;color:#0A0A0A;text-align:left;cursor:pointer;box-shadow:0 4px 0 #8A8A8A,0 7px 0 #5A5A5A,0 14px 24px rgba(0,0,0,.5);transform-origin:50% 100%;transition:transform .1s,box-shadow .1s}
.go .in{display:flex;align-items:center;gap:12px;border:1.5px solid #0A0A0A;border-radius:8px;padding:6px 14px;height:42px;box-sizing:border-box}
.go .t{display:block;font-family:'Cinzel',Georgia,serif;font-weight:800;font-size:17px;line-height:1.05}
.go .s{display:block;font-size:11px;font-weight:600;color:#5A5A5A;margin-top:2px}
.go .chev{margin-left:auto;font-family:'Cinzel',serif;font-weight:800;font-size:22px}
.go:active{transform:perspective(500px) rotateX(14deg) translateY(4px);box-shadow:0 1px 0 #8A8A8A,0 2px 0 #5A5A5A}
.go.off{background:#141414;color:#8C8C8C;box-shadow:0 0 0 1.5px #2C2C2C;cursor:default;pointer-events:none}
.go.off .in{border:1.5px dashed #3A3A3A}
.go.off .s{color:#6E6E6E}
.btn{font:inherit;font-family:'Cinzel',Georgia,serif;font-size:12px;font-weight:800;color:#F2F2F2;background:#141414;border:0;border-radius:8px;height:34px;padding:0 12px;display:inline-flex;align-items:center;justify-content:center;gap:6px;cursor:pointer;white-space:nowrap;outline:1.5px solid #5A5A5A;outline-offset:-4px;box-shadow:0 0 0 1.5px #3A3A3A,0 3px 0 #000;flex:none}
.btn:active{transform:translateY(2px)}
.panel{background:linear-gradient(180deg,#181818,#111);border:1px solid #2C2C2C;border-radius:14px;box-shadow:inset 0 1px 0 rgba(255,255,255,.06),0 12px 30px rgba(0,0,0,.45)}
.ph{display:flex;justify-content:space-between;align-items:baseline;padding:12px 14px 8px}
.hd{display:flex;align-items:baseline;gap:14px}
.sec{display:flex;flex-direction:column;gap:8px}
.camps{display:grid;grid-template-columns:repeat(4,1fr);gap:18px}
.camp{display:flex;flex-direction:column;gap:10px;padding:8px;border-radius:14px;background:#EDEDED;color:#0A0A0A;box-shadow:0 4px 0 #8A8A8A,0 7px 0 #5A5A5A,0 16px 28px rgba(0,0,0,.5);height:440px;box-sizing:border-box}
.camp .fr{flex:1;display:flex;flex-direction:column;gap:8px;border:1.5px solid #0A0A0A;border-radius:9px;padding:10px 12px}
.camp .art{height:200px;border-radius:7px;display:flex;align-items:flex-end;justify-content:center;gap:4px;padding-bottom:12px;box-sizing:border-box;background:radial-gradient(70% 60% at 50% 100%,rgba(255,255,255,.12),transparent),repeating-linear-gradient(135deg,#2E2E2E 0 2px,#1C1C1C 2px 9px)}
.camp .meta{font-size:12px;color:#5A5A5A;line-height:1.5}
.camp.dk{background:#141414;color:#F2F2F2;box-shadow:0 0 0 1.5px #3A3A3A,0 4px 0 #000}
.camp.dk .fr{border-color:#5A5A5A}
.camp.dk .meta{color:#8C8C8C}
.camp.new{background:transparent;box-shadow:none;border:2px dashed #5E5E5E;color:#D4D4D4;align-items:center;justify-content:center;text-align:center}
.camp.new.on{border-color:#F2F2F2;color:#F2F2F2;box-shadow:0 0 30px rgba(255,255,255,.12)}
.unis{display:grid;grid-template-columns:repeat(3,1fr);gap:18px}
.uni{font:inherit;text-align:left;display:flex;flex-direction:column;gap:12px;padding:14px;border-radius:14px;background:#141414;border:1.5px solid #2C2C2C;color:#F2F2F2;cursor:pointer}
.uni.on{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A;outline:1.5px solid #0A0A0A;outline-offset:-7px;box-shadow:0 4px 0 #8A8A8A,0 16px 28px rgba(0,0,0,.5)}
.uni .mapw{position:relative;isolation:isolate;height:180px;overflow:hidden;border-radius:9px;border:1px solid #2C2C2C}
.uni .row{display:flex;gap:8px;align-items:flex-end}
.uni .st{display:flex;flex-wrap:wrap;gap:6px}
.uni .st > span{font-size:11px;font-weight:600;padding:3px 8px;border-radius:999px;border:1px solid #3A3A3A;color:#A3A3A3}
.uni.on .st > span{border-color:#0A0A0A;color:#0A0A0A}
.uni .ds{font-size:13px;line-height:1.45;color:#A3A3A3}
.uni.on .ds{color:#3A3A3A}
.two{display:grid;gap:18px;min-height:0}
.opt{font:inherit;text-align:left;display:flex;flex-direction:column;gap:3px;padding:14px 16px;border-radius:12px;background:#141414;border:1.5px solid #2C2C2C;color:#F2F2F2}
.opt > b{font-family:'Cinzel',serif;font-size:17px}
.opt > span{font-size:12px;color:#A3A3A3;line-height:1.45}
.opt.on{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A;outline:1.5px solid #0A0A0A;outline-offset:-5px;box-shadow:0 3px 0 #8A8A8A}
.opt.on > span{color:#5A5A5A}
.rl{display:grid;grid-template-columns:130px 1fr;gap:12px;align-items:baseline;font-size:13px;line-height:1.45;padding:9px 12px;border-radius:10px;background:#121212;color:#D4D4D4}
.hand{display:flex;gap:14px}
.edit{position:relative;padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1px solid #333}
.edit > .lbl{display:block;margin-bottom:8px}
.caret{display:inline-block;width:2px;height:18px;background:#F5F5F5;margin-left:2px;vertical-align:-3px;animation:caret .8s steps(1) infinite}
@keyframes caret{50%{opacity:0}}
.tags{display:flex;gap:8px;flex-wrap:wrap}
.tag{font-size:12px;font-weight:600;padding:6px 12px;border-radius:999px;border:1px solid #333;color:#A3A3A3;background:#141414}
.tag.on{background:#F2F2F2;color:#0A0A0A;border-color:#F2F2F2}
.est{display:flex;flex-direction:column;gap:8px;padding:14px 16px;border-radius:12px;background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.est .lbl{color:#5A5A5A}
.est .ln{display:flex;justify-content:space-between;font-size:13px}
.est .tot{display:flex;justify-content:space-between;font-weight:700;font-size:16px;border-top:1.5px solid #0A0A0A;padding-top:8px}
.gs{display:grid;grid-template-columns:22px 130px 1fr 70px;gap:12px;align-items:center;height:46px;padding:0 14px;border-radius:10px;background:#121212;font-size:13px;color:#6E6E6E}
.gs > i{width:12px;height:12px;transform:rotate(45deg);border:1.5px solid currentColor;box-sizing:border-box}
.gs.done{color:#D4D4D4}
.gs.done > i{background:#D4D4D4}
.gs.cur{background:#1C1C1C;color:#F2F2F2;border:1px solid #3A3A3A}
.gs > em{font-style:normal;font-size:12px;text-align:right}
.ai{display:flex;align-items:center;gap:10px;font-size:13px;color:#D4D4D4}
.col3{display:grid;grid-template-columns:340px 1fr 320px;gap:14px;min-height:0;flex:1}
.chat{display:flex;flex-direction:column;gap:12px;padding:0 14px 14px;overflow:hidden}
.me{align-self:flex-end;max-width:88%;background:#1C1C1C;border-radius:12px 12px 4px 12px;padding:9px 12px;font-size:13px;line-height:1.5}
.it{max-width:94%;font-size:13px;line-height:1.55;color:#D4D4D4}
.diff{background:#EDEDED;color:#0A0A0A;border-radius:12px;box-shadow:0 3px 0 #8A8A8A,0 12px 24px rgba(0,0,0,.5);overflow:hidden}
.diff .dh{display:flex;justify-content:space-between;padding:9px 12px;border-bottom:1.5px solid #0A0A0A;font-size:12px}
.diff .db{font-size:12px;line-height:1.6;padding:9px 12px;display:flex;flex-direction:column;gap:6px}
.diff .db small{opacity:.65}
.diff .df{display:flex;gap:8px;padding:9px 12px;border-top:1.5px solid #0A0A0A}
.diff.ok{background:#141414;color:#A3A3A3;box-shadow:0 0 0 1px #2C2C2C}
.diff.ok .dh,.diff.ok .df{border-color:#2C2C2C}
.tbl{position:relative;width:690px;height:440px;margin:0 auto}
.node{position:absolute;width:180px;height:88px;box-sizing:border-box;border-radius:11px;background:#EDEDED;color:#0A0A0A;border:0;padding:5px;text-align:left;font:inherit;cursor:pointer;box-shadow:0 3px 0 #8A8A8A,0 5px 0 #5A5A5A,0 12px 22px rgba(0,0,0,.55)}
.node > span{display:flex;flex-direction:column;gap:2px;height:100%;box-sizing:border-box;border:1.5px solid #0A0A0A;border-radius:7px;padding:6px 9px}
.node.on{box-shadow:0 0 0 3px #0A0A0A,0 0 0 5px #F2F2F2,0 16px 26px rgba(0,0,0,.6);transform:translateY(-4px)}
.node.gen{background:#141414;color:#F2F2F2;box-shadow:0 3px 0 #000}
.node.gen > span{border:1.5px dashed #8C8C8C}
.edge{position:absolute;background:#3A3A3A}
.edge.hot{background:#F2F2F2}
.det{display:flex;flex-direction:column;gap:12px;padding:0 14px 14px;overflow:hidden}
.ill{height:120px;border-radius:10px;position:relative;background:radial-gradient(80% 90% at 50% 100%,rgba(10,10,10,.8),transparent),repeating-linear-gradient(135deg,#3A3A3A 0 2px,#262626 2px 8px)}
.ill > span{position:absolute;left:10px;bottom:10px;font-size:9px;font-weight:700;letter-spacing:.14em;background:#0A0A0A;padding:3px 6px;border-radius:4px}
.cl{display:flex;flex-direction:column;gap:2px;padding:7px 10px;background:#161616;border-radius:8px;font-size:12px}
.cl > small{font-size:11px;color:#8A8A8A}
.cl.miss{background:transparent;border:1.5px dashed #FF9F1C;color:#F2F2F2}
.cl.add{background:#EDEDED;color:#0A0A0A}
.cl.add > small{color:#5A5A5A}
.chk{display:flex;align-items:center;gap:12px;font-size:13px;padding:12px 14px;border-radius:10px;background:#121212;line-height:1.45}
.chk > i{width:20px;height:20px;border-radius:5px;background:#EDEDED;color:#0A0A0A;display:grid;place-items:center;font-style:normal;font-weight:800;font-size:12px;flex:none}
.chk.warn{border:1.5px solid #FF9F1C}
.chk.warn > i{background:#FF9F1C}
.chk.later{border:1.5px dashed #5E5E5E;color:#A3A3A3}
.chk.later > i{background:transparent;color:#8C8C8C;border:1.5px dashed #5E5E5E;box-sizing:border-box}
.chk > span:last-child{margin-left:auto;font-size:11px;color:#8C8C8C;white-space:nowrap}
.fl{display:flex;flex-direction:column;gap:3px;padding:0 10px 10px}
.fi{display:flex;align-items:center;gap:10px;height:36px;padding:0 10px;border-radius:8px;font-size:13px;color:#D4D4D4}
.fi > small{margin-left:auto;font-size:11px;color:#8C8C8C}
.fi.on{background:#EDEDED;color:#0A0A0A;font-weight:700}
.fi.on > small{color:#5A5A5A}
.sheet{display:grid;grid-template-columns:200px 1fr;gap:22px}
.stage{display:grid;place-items:end center;height:230px;border-radius:14px;background:radial-gradient(60% 30% at 50% 92%,rgba(255,255,255,.1),transparent),#141414;border:1px solid #2C2C2C;padding-bottom:16px;box-sizing:border-box}
.gems{display:flex;gap:16px;align-items:flex-end}
.gem{display:flex;flex-direction:column;align-items:center;gap:4px;font-size:10px;font-weight:600;letter-spacing:.12em;text-transform:uppercase;color:#8C8C8C}
.mt{display:grid;grid-template-columns:200px 120px 140px 1fr 70px;align-items:center;gap:12px;height:46px;padding:0 14px;border-radius:10px;background:#121212;font-size:13px;color:#D4D4D4}
.mt.on{background:#1C1C1C;border:1px solid #F2F2F2}
.mt.hd{background:none;height:24px;font-size:10px;font-weight:600;letter-spacing:.16em;text-transform:uppercase;color:#8C8C8C}
.mt > em{font-style:normal;text-align:right;color:#8C8C8C;font-size:12px}
.mt > .ok{color:#F2F2F2}
.mt > .todo{color:#FF9F1C}
.mt > .pend{color:#8C8C8C}
.mapw{position:relative;isolation:isolate;overflow:hidden;border-radius:10px;border:1px solid #2C2C2C}
.sum{display:grid;grid-template-columns:repeat(5,1fr);gap:14px}
.num{display:flex;flex-direction:column;gap:4px;padding:16px;border-radius:12px;background:#141414;border:1px solid #2C2C2C}
.num > b{font-family:'Cinzel',serif;font-size:34px;line-height:1}
.num > span{font-size:12px;color:#A3A3A3}
.inv{display:flex;align-items:center;gap:14px;padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1px solid #333;font-size:14px}
.inv > code{flex:1;font-family:'Chakra Petch',monospace;font-size:14px;color:#F2F2F2}
.endc{position:absolute;inset:0;z-index:70;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:16px;text-align:center;background:rgba(5,5,5,.94)}
.side{display:flex;flex-direction:column;gap:18px;width:520px;flex:none}
.think{padding:14px 16px;border-radius:14px;background:#EDEDED;color:#0A0A0A;font-size:14px;line-height:1.45;box-shadow:0 3px 0 #8A8A8A;min-height:92px;box-sizing:border-box;display:flex;align-items:center}
.why{padding:12px 14px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.5;color:#D4D4D4;min-height:92px;box-sizing:border-box}
.prog{display:grid;grid-template-columns:repeat(10,1fr);gap:6px}
.prog > button{font:inherit;font-size:13px;font-weight:700;height:36px;border-radius:8px;background:#141414;border:1px solid #2C2C2C;color:#6E6E6E;cursor:pointer}
.prog > button.done{color:#A3A3A3}
.prog > button.cur{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.ctl{display:flex;gap:10px}
.ctl > button{font:inherit;font-size:13px;font-weight:600;height:40px;padding:0 16px;border-radius:10px;background:#141414;border:1px solid #3A3A3A;color:#D4D4D4;cursor:pointer}
@media (prefers-reduced-motion: reduce){.pm *{animation:none!important}}
'''


def imp(name, size, **kw):
    a = ' '.join(f'{k}="{v}"' for k, v in kw.items())
    return f'<dc-import name="{name}" {a} hint-size="{size[0]}px,{size[1]}px"></dc-import>'


def block(n, body):
    return '<sc-if value="{{is%d}}" hint-placeholder-val="{{ %s }}"><div class="body">%s</div></sc-if>' % (n, 'true' if n == 1 else 'false', body)


def head(title, lbl):
    return f'<div class="hd"><span class="ttl" style="font-size: 26px">{title}</span><span class="lbl">{lbl}</span></div>'


B = []
# 1 · campaign list
B.append(block(1, head('Mes campagnes', 'Romain · MJ')
  + '<div class="camps">'
  + '<div class="camp"><div class="fr"><div class="art">' + ''.join(imp('Sprite', (40, 52), perso=p, px=2, anim='repos') for p in ['lyra', 'borin', 'sef']) + '</div><span class="lbl" style="color: #5A5A5A">Fantasy · terminée</span><span class="ttl" style="font-size: 20px">Le phare de Kerlann</span><span class="meta">6 sessions · Camille, Marc, Hugo<br>Dernière : 12 septembre</span></div></div>'
  + '<div class="camp dk"><div class="fr"><div class="art">' + ''.join(imp('Sprite', (40, 52), perso=p, px=2, anim='repos') for p in ['soldat', 'mara', 'zombie']) + '</div><span class="lbl">Zombies · brouillon</span><span class="ttl" style="font-size: 20px">Hôpital Saint-Roch</span><span class="meta">Pitch écrit, rien de généré<br>Commencée le 2 août</span></div></div>'
  + '<div class="camp new {{newCls}}"><span class="ttl" style="font-size: 54px; line-height: 1">+</span><span class="ttl" style="font-size: 20px">Nouvelle campagne</span><span style="font-size: 13px; color: #8C8C8C; max-width: 220px; line-height: 1.45">Un univers, des règles, un pitch. Le co-MJ écrit le reste, tu relis.</span></div>'
  + '<div class="camp new" style="border-style: solid; border-color: #1F1F1F; color: #5A5A5A"><span style="font-size: 13px; line-height: 1.5; max-width: 220px">Les campagnes terminées gardent leur chronique et leurs personnages.</span></div>'
  + '</div>'))
# 2 · universe
UNIS = [('fantasy', 'Fantasy', 'crypte', ['borin', 'lyra', 'gobelin'], ['Vie', 'Attaque', 'Armure', 'Magie', 'Mouvement', 'Initiative'], 'Cryptes, forêts, villages. Nains, elfes, gobelins. Haches et parchemins.'),
        ('zombies', 'Zombies', 'hopital', ['soldat', 'mara', 'zombie'], ['Vie', 'Attaque', 'Protection', 'Sang-froid', 'Mouvement', 'Réflexes'], 'Hôpitaux, rues, centres commerciaux. Survivants et morts qui marchent.'),
        ('spatial', 'Spatial', 'vaisseau', ['alien', 'soldat', 'mara'], ['Santé', 'Tir', 'Bouclier', 'Psi', 'Propulsion', 'Réflexes'], 'Coursives, stations, planètes. Équipages, androïdes, créatures.')]
B.append(block(2, head('L’univers', 'il fixe les décors, les personnages, les objets et le nom des stats')
  + '<div class="unis">'
  + ''.join('<button type="button" class="uni {{u%d}}" onClick="{{pickU%d}}">' % (i, i)
            + f'<div class="mapw" style="height: 200px; background: #050505"><div style="position: absolute; left: 0; top: -30px">{imp("Plan", (420, 300), theme=th, carte="salle", pitch=30, vue="joueur", grille="false")}</div></div>'
            + f'<div class="hd" style="justify-content: space-between"><span class="ttl" style="font-size: 22px">{n}</span><span class="row">' + ''.join(imp('Sprite', (40, 52), perso=p, px=2, anim='repos') for p in ps) + '</span></div>'
            + f'<span class="ds">{d}</span><span class="st">' + ''.join(f'<span>{s}</span>' for s in st) + '</span></button>'
            for i, (k, n, th, ps, st, d) in enumerate(UNIS))
  + '</div><span style="font-size: 12px; color: #8C8C8C">Un univers est un pack : tuiles, morceaux de personnages, objets et noms des six stats. Le moteur, la grille et les règles restent les mêmes.</span>'))
# 3 · rules
B.append(block(3, head('Les règles', 'les actions des joueurs en découlent')
  + '<div class="two" style="grid-template-columns: 380px 1fr"><div class="sec"><span class="lbl">Partir de</span>'
  + '<div class="opt on"><b>D&amp;D 5e (SRD)</b><span>d20 + modificateur contre une difficulté. Six stats, classes, sorts. Le plus connu des joueurs.</span></div>'
  + '<div class="opt"><b>Allégé</b><span>Trois stats, un d6. Pour une soirée avec des débutants.</span></div>'
  + '<div class="opt"><b>Vide</b><span>Tu écris ton système. Le co-MJ t’aide à le tenir cohérent.</span></div>'
  + '<span style="font-size: 12px; color: #8C8C8C; line-height: 1.5">Tout se modifie plus tard dans l’onglet Règles, même en cours de campagne.</span></div>'
  + '<div class="sec"><span class="lbl">Ce que verront les joueurs</span><div class="hand">'
  + imp('GameCard', (150, 210), titre='Hache d’armes', type='Arme', texte='1d12 + 3 dégâts.', valeur='+5', stat='atq', icon='epee', rarete='commune', w=150)
  + imp('GameCard', (150, 210), titre='Second souffle', type='Compétence', texte='Soigne 1d10 + 3.', valeur='d10', stat='pv', icon='sort', rarete='peu-commune', w=150)
  + imp('GameCard', (150, 210), titre='Fouiller', type='Sagesse', texte='Chercher, observer.', valeur='+4', stat='none', icon='oeil', rarete='commune', w=150)
  + imp('GameCard', (150, 210), titre='Se déplacer', type='Mouvement', texte='6 cases par tour.', valeur='6', stat='mvt', icon='pas', rarete='commune', w=150)
  + '</div>'
  + '<div class="rl"><span class="lbl">Un jet</span><span>d20 + la stat, contre une difficulté que tu fixes (10 facile, 13 moyen, 16 dur).</span></div>'
  + '<div class="rl"><span class="lbl">Le combat</span><span>Initiative, un tour chacun. Le serveur vérifie portée, ligne de vue et déplacement.</span></div>'
  + '<div class="rl"><span class="lbl">Les cartes</span><span>Monde en hexagones de 10 km, lieux en cases de 5 m, rencontres en cases de 1,5 m.</span></div>'
  + '</div></div>'))
# 4 · pitch
B.append(block(4, head('Le pitch', 'quelques phrases suffisent')
  + '<div class="two" style="grid-template-columns: 1fr 420px"><div class="sec">'
  + '<div class="edit"><span class="lbl">Ton idée</span><p class="nar" style="font-size: 24px">Une ville minière qui enterre ses morts deux fois. Sous la colline, une crypte que personne n’ose ouvrir. Un maire qui cache quelque chose.<span class="caret"></span></p></div>'
  + '<span class="lbl">Ton</span><div class="tags"><span class="tag on">Gothique</span><span class="tag">Héroïque</span><span class="tag">Léger</span><span class="tag">Enquête</span><span class="tag on">Mystère</span></div>'
  + '<div class="rl"><span class="lbl">Joueurs</span><span>3 · Camille, Marc, Hugo · débutants acceptés</span></div>'
  + '<div class="rl"><span class="lbl">Durée</span><span>5 à 6 sessions de 2 h 30</span></div>'
  + '<div class="rl"><span class="lbl">Niveau</span><span>Les personnages commencent au niveau 1</span></div>'
  + '</div><div class="sec"><div class="est"><span class="lbl">Avant de lancer · estimation</span>'
  + '<div class="ln"><span>Bible et fronts</span><span>0,28 $</span></div><div class="ln"><span>8 scènes et leurs indices</span><span>0,54 $</span></div><div class="ln"><span>14 fiches (PNJ, monstres, objets)</span><span>0,30 $</span></div><div class="ln"><span>Vérification de cohérence</span><span>0,04 $</span></div>'
  + '<div class="tot"><span>Total</span><span>≈ 1,16 $</span></div></div>'
  + '<div class="rl" style="grid-template-columns: 110px 1fr"><span class="lbl">Budget IA</span><span>10 $ pour la campagne. Les images, cartes et vidéos se chiffrent à part, plus tard, lot par lot.</span></div></div></div>'))
# 5 · generation
B.append(block(5, head('Le co-MJ écrit', 'tu peux lire ce qui est fini')
  + '<div class="two" style="grid-template-columns: 520px 1fr"><div class="sec"><sc-for list="{{gen}}" as="g" hint-placeholder-count="6"><div class="gs {{g.cls}}"><i></i><span>{{g.n}}</span><span>{{g.d}}</span><em>{{g.c}}</em></div></sc-for>'
  + '<div style="display: flex; gap: 10px; align-items: center"><span style="width: 60px; height: 20px; overflow: hidden; border-radius: 4px">' + imp('RadialGrid', (60, 20), cols=6, rows=2, pitch=10, pattern='vague', ox=0) + '</span><span style="font-size: 13px; color: #D4D4D4; flex: 1">{{genNow}}</span><button type="button" class="btn">Interrompre</button></div></div>'
  + '<div class="sec"><span class="lbl">Déjà écrit · la bible</span><div class="edit"><p class="nar" style="font-size: 19px">Valombre vit de sa mine d’argent et de ses morts. Depuis un siècle, on y enterre deux fois : une fois au cimetière, une fois sous la colline, dans la crypte des Valombre. Le culte des cendres veut réveiller ceux d’en bas.</p></div>'
  + '<sc-if value="{{frontsOn}}" hint-placeholder-val="{{ true }}"><span class="lbl">Les fronts · ce qui arrive si personne n’agit</span><div style="display: flex; gap: 26px">'
  + '<div class="ai">' + imp('Horloge', (64, 64), parts=6, rempli=0, taille=64, texte='false') + '<span><b style="color: #F2F2F2">Le culte des cendres</b><br>6 étapes, jusqu’au réveil</span></div>'
  + '<div class="ai">' + imp('Horloge', (64, 64), parts=4, rempli=0, taille=64, texte='false') + '<span><b style="color: #F2F2F2">La milice du maire</b><br>4 étapes, jusqu’au couvre-feu</span></div></div></sc-if></div></div>'))
# 6 · the workshop
WORK = ('<div class="col3"><section class="panel" style="display: flex; flex-direction: column; min-height: 0"><div class="ph"><span class="lbl" style="color: #F2F2F2">Co-MJ</span><span class="lbl">{{costTxt}}</span></div><div class="chat">'
  + '<div class="me">Rends le maire plus ambigu, et ajoute un indice dans la crypte.</div>'
  + '<div class="it">Le maire Aldric ne sert pas le culte : il le paie pour que les morts restent sous terre. J’ajoute un indice qui pointe vers lui sans l’accuser.</div>'
  + '<div class="diff {{diffCls}}"><div class="dh"><b class="ttl">{{diffTtl}}</b><span>0,03 $</span></div><div class="db"><span><b>+</b> Indice « Registre des sépultures »<br><small>→ nœud La crypte des Valombre</small></span><span><b>~</b> PNJ Aldric · motivation<br><small>« protéger la ville » → « acheter le silence »</small></span></div>'
  + '<sc-if value="{{diffOpen}}" hint-placeholder-val="{{ true }}"><div class="df"><button type="button" class="btn">Modifier</button><button type="button" class="btn">Rejeter</button><span class="lbl" style="margin-left: auto; align-self: center; color: #5A5A5A">ou accepte en bas</span></div></sc-if></div>'
  + '<div class="tags" style="margin-top: auto"><span class="tag">Ajouter une scène</span><span class="tag">Durcir une menace</span><span class="tag">Vérifier les indices</span></div></div></section>'
  + '<section class="panel" style="min-height: 0; overflow: hidden"><div class="ph"><span class="lbl" style="color: #F2F2F2">Graphe · 8 scènes en 3 actes</span><span class="lbl">touche une scène</span></div><div class="tbl">'
  + '<span class="lbl" style="position: absolute; left: 12px; top: 0">Acte I</span><span class="lbl" style="position: absolute; left: 258px; top: 0">Acte II</span><span class="lbl" style="position: absolute; left: 504px; top: 0">Acte III</span>'
  + '<sc-for list="{{edges}}" as="e" hint-placeholder-count="12"><div class="edge {{e.cls}}" style="left: {{e.l}}px; top: {{e.t}}px; width: {{e.w}}px; height: {{e.h}}px"></div></sc-for>'
  + '<sc-for list="{{nodes}}" as="n" hint-placeholder-count="8"><button type="button" class="node {{n.cls}}" onClick="{{n.pick}}" style="left: {{n.x}}px; top: {{n.y}}px"><span><span style="font-size: 9px; font-weight: 700; letter-spacing: .16em; text-transform: uppercase; opacity: .7">{{n.kind}}</span><span class="ttl" style="font-size: 13px; line-height: 1.15">{{n.title}}</span><span style="font-size: 10px; font-weight: 600; opacity: .75; margin-top: auto">{{n.meta}}</span></span></button></sc-for>'
  + '</div></section>'
  + '<section class="panel" style="display: flex; flex-direction: column; min-height: 0"><div class="ph"><span class="lbl">{{sel.kind}} · {{sel.act}}</span><span class="lbl">{{sel.map}}</span></div><div class="det">'
  + '<div class="ill"><span>[ILLUSTRATION · À GÉNÉRER]</span></div><span class="ttl" style="font-size: 20px">{{sel.title}}</span><span style="font-size: 12px; color: #A3A3A3">Objectif : {{sel.goal}}</span>'
  + '<p class="nar" style="font-size: 17px">{{sel.read}}</p><span class="lbl">Indices</span><sc-for list="{{sel.clues}}" as="c" hint-placeholder-count="2"><div class="cl"><span>{{c.name}}</span><small>→ {{c.to}}</small></div></sc-for>'
  + '<div style="display: flex; gap: 8px; margin-top: auto"><button type="button" class="btn" style="flex: 1">Régénérer</button><button type="button" class="btn" style="flex: 1">Éditer</button></div></div></section></div>')
B.append(block(6, WORK))
# 7 · coherence
B.append(block(7, head('Cohérence', 'vérifiée par le co-MJ · 0,01 $')
  + '<div class="two" style="grid-template-columns: 560px 1fr"><div class="sec">'
  + '<div class="chk"><i>✓</i><span>Chaque scène a au moins une sortie</span><span>8 sur 8</span></div>'
  + '<div class="chk"><i>✓</i><span>Chaque front a un déclencheur dans une scène</span><span>2 sur 2</span></div>'
  + '<div class="chk {{alertCls}}"><i>{{alertMark}}</i><span>Chaque révélation a au moins 3 indices</span><span>{{alertCount}}</span></div>'
  + '<div class="chk later"><i>?</i><span>Capitaine Brune n’apparaît que sur la place du marché</span><span>à voir plus tard</span></div>'
  + '<span style="font-size: 12px; color: #8C8C8C; line-height: 1.5">La règle des trois indices : si les joueurs ratent un indice, deux autres mènent à la même révélation. Personne ne reste bloqué.</span></div>'
  + '<section class="panel" style="padding: 0 0 14px"><div class="ph"><span class="lbl" style="color: #F2F2F2">Révélation</span><span class="lbl">{{alertCount}}</span></div><div class="det" style="padding-bottom: 0">'
  + '<span class="ttl" style="font-size: 22px">Les gobelins travaillent pour quelqu’un</span>'
  + '<div class="cl"><span>Empreintes de petites bottes</span><small>Le cimetière sous la pluie</small></div>'
  + '<div class="cl"><span>Bourse pleine d’argent neuf</span><small>La crypte des Valombre · sur le chef gobelin</small></div>'
  + '<sc-if value="{{notFixed}}" hint-placeholder-val="{{ true }}"><div class="cl miss"><span>Il manque un troisième indice</span><small>Le co-MJ propose ci-dessous</small></div>'
  + '<div class="diff"><div class="dh"><b class="ttl">Proposition · 1 changement</b><span>0,02 $</span></div><div class="db"><span><b>+</b> Indice « Ordre de paie au sceau du maire »<br><small>→ nœud Le manoir du maire · dans le bureau, sous clé</small></span></div><div class="df"><button type="button" class="btn">Ailleurs</button><button type="button" class="btn">Rejeter</button></div></div></sc-if>'
  + '<sc-if value="{{fixed}}" hint-placeholder-val="{{ false }}"><div class="cl add"><span>Ordre de paie au sceau du maire</span><small>Le manoir du maire · ajouté</small></div></sc-if>'
  + '</div></section></div>'))
# 8 · sheets
FICHES = [('PNJ', [('Maire Aldric', 'manoir'), ('Mère Ysolde', 'auberge'), ('Tobin le fossoyeur', 'cimetière'), ('Capitaine Brune', 'marché'), ('Grande prêtresse', 'rituel')]),
          ('Monstres', [('Gobelin', '×2 · crypte'), ('Chef gobelin', 'crypte · caché'), ('Mort qui marche', 'rituel')]),
          ('Objets', [('Hache des Valombre', 'légendaire'), ('Bague du gardien', 'peu commune'), ('Registre', 'indice'), ('Page arrachée', 'indice'), ('Potion de soin', '×4'), ('Clé du manoir', 'indice')])]
fl = ''
for g, items in FICHES:
    fl += f'<span class="lbl" style="padding: 8px 10px 2px">{g} · {len(items)}</span>'
    for n, w in items:
        fl += f'<div class="fi {"on" if n == "Chef gobelin" else ""}"><span>{n}</span><small>{w}</small></div>'
B.append(block(8, '<div class="two" style="grid-template-columns: 300px 1fr; flex: 1"><section class="panel" style="min-height: 0; overflow: hidden"><div class="ph"><span class="lbl" style="color: #F2F2F2">Fiches · 14</span><span class="lbl">relues 13</span></div><div class="fl">' + fl + '</div></section>'
  + '<div class="sec" style="gap: 14px">' + head('Chef gobelin', 'monstre · fiche écrite par le co-MJ')
  + '<div class="sheet"><div class="stage">' + imp('Perso', (120, 156), perso='chef', px=6, etat='aucun') + '</div><div class="sec" style="gap: 12px">'
  + '<div class="gems">' + ''.join(f'<span class="gem">{imp("Gemme", (44, 44), stat=s, valeur=v, taille=44, anim="false")}{n}</span>' for s, v, n in [('pv', '21', 'Vie'), ('ca', '16', 'Armure'), ('atq', '+5', 'Attaque'), ('mvt', '6', 'Mouvement'), ('ini', '+2', 'Initiative')]) + '</div>'
  + '<div class="rl"><span class="lbl">Ce qu’il veut</span><span>Garder la porte est. Le culte le paie en argent neuf, il ne pose pas de questions.</span></div>'
  + '<div class="rl"><span class="lbl">Où</span><span>La crypte des Valombre, caché derrière la porte est. Il n’entre en jeu que si le groupe ouvre cette porte.</span></div>'
  + '<div class="rl"><span class="lbl">Sur lui</span><span style="display: flex; align-items: center; gap: 10px">' + imp('Objet', (40, 40), objet='bourse', rarete='commune', taille=40, pips='false') + 'Bourse pleine d’argent neuf (indice)</span></div></div></div>'
  + '<div class="hand">' + imp('GameCard', (110, 154), titre='Cimeterre', type='Arme', texte='1d6 + 3 dégâts.', valeur='+5', stat='atq', icon='epee', rarete='commune', w=110)
  + imp('GameCard', (110, 154), titre='Ralliement', type='Cri', texte='Les gobelins attaquent deux fois.', valeur='', stat='none', icon='parole', rarete='rare', w=110)
  + '<div class="sec" style="flex: 1; justify-content: flex-end"><span style="font-size: 12px; color: #8C8C8C; line-height: 1.5">Ses PV et son nom restent cachés aux joueurs tant que tu ne le révèles pas.</span><div style="display: flex; gap: 8px"><button type="button" class="btn">Modifier</button><button type="button" class="btn">Régénérer</button><button type="button" class="btn">Créer un PNJ</button></div></div></div></div></div>'))
# 9 · maps and media
B.append(block(9, head('Cartes et médias', 'par lieu · rien n’est généré sans ton accord')
  + '<div class="two" style="grid-template-columns: 1fr 380px"><div class="sec" style="gap: 6px">'
  + '<div class="mt hd"><span>Lieu</span><span>Carte</span><span>Illustration</span><span>Musique</span><em>Coût</em></div>'
  + '<sc-for list="{{media}}" as="r" hint-placeholder-count="6"><div class="mt {{r.cls}}"><span>{{r.n}}</span><span class="{{r.mc}}">{{r.m}}</span><span class="{{r.ic}}">{{r.i}}</span><span>{{r.y}}</span><em>{{r.c}}</em></div></sc-for>'
  + '<span style="font-size: 12px; color: #8C8C8C; line-height: 1.5; margin-top: 6px">Les cartes sont proposées par le co-MJ dans le jeu de tuiles de l’univers ; tu les retouches dans l’éditeur. La musique est un lien YouTube que tu choisis.</span></div>'
  + '<section class="panel" style="padding: 0 0 14px"><div class="ph"><span class="lbl" style="color: #F2F2F2">La crypte des Valombre</span><span class="lbl">rencontre</span></div><div class="det" style="padding-bottom: 0">'
  + '<div class="mapw" style="height: 216px"><div style="position: absolute; left: 0; top: -24px">' + imp('Plan', (336, 240), theme='crypte', carte='salle', pitch=24, vue='mj', grille='true') + '</div></div>'
  + '<div class="ill" style="height: 90px"><span>{{crIll}}</span></div><div class="ai"><span>Musique : Ombres de la crypte · YouTube</span></div>'
  + '<div style="display: flex; gap: 8px"><button type="button" class="btn" style="flex: 1">Ouvrir l’éditeur</button><button type="button" class="btn" style="flex: 1">Importer une image</button></div></div></section></div>'))
# 10 · validate
B.append(block(10, head('Valider la campagne', 'elle devient jouable')
  + '<div class="sum">' + ''.join(f'<div class="num"><b>{n}</b><span>{t}</span></div>' for n, t in [('8', 'scènes en 3 actes'), ('14', 'fiches relues'), ('3', 'révélations, 3 indices chacune'), ('2', 'fronts et leurs horloges'), ('5', 'cartes prêtes sur 6')]) + '</div>'
  + '<div class="two" style="grid-template-columns: 1fr 1fr"><div class="sec"><span class="lbl">Inviter les joueurs</span><div class="inv"><code>Lien d’invitation · Les Cendres de Valombre</code><button type="button" class="btn">Copier</button></div>'
  + '<span style="font-size: 12px; color: #8C8C8C; line-height: 1.5">Pas de compte pour les joueurs : un lien, un pseudo, un personnage. Tu valides chaque personnage avant la session 1.</span></div>'
  + '<div class="sec"><span class="lbl">Première session</span><div class="inv"><span class="ttl" style="font-size: 18px; flex: 1">Jeudi 19 septembre · 20 h 30</span><button type="button" class="btn">Changer</button></div>'
  + '<div class="rl"><span class="lbl">Coût total</span><span>{{spentTxt}} sur 10 $ · il reste de quoi générer les médias des actes II et III</span></div></div></div>'
  + '<div class="rl"><span class="lbl">Encore à faire</span><span>Capitaine Brune n’a pas de rôle (noté pour plus tard). Les illustrations de l’acte III viendront quand le groupe s’en approchera.</span></div>'))

TOP = ('<header class="top"><span class="ttl" style="font-size: 13px; letter-spacing: .3em; color: #F2F2F2">PROMPTUS</span><span class="sep"></span>'
       '<span class="ttl" style="font-size: 15px; color: #F2F2F2">{{campName}}</span><span class="chipst">{{campState}}</span><span style="flex: 1"></span>'
       '<div class="budget"><div style="display: flex; justify-content: space-between"><span class="lbl">Budget IA</span><span style="font-size: 12px; font-weight: 600; color: #F2F2F2">{{spentTxt}} / 10 $</span></div><div class="bar"><i style="width: {{spentPct}}%"></i></div></div></header>')
STEPS = '<nav class="steps" aria-label="Étapes de préparation"><sc-for list="{{steps}}" as="s" hint-placeholder-count="9"><sc-if value="{{s.rail}}" hint-placeholder-val="{{ true }}"><span class="rail"></span></sc-if><span class="sp {{s.cls}}"><i></i>{{s.n}}</span></sc-for></nav>'
FOOT = '<div class="foot"><div class="bn {{bnCls}}"><span>{{bnTxt}}</span></div><button type="button" class="go {{mainCls}}" onClick="{{mainAct}}"><span class="in"><span style="flex: 1"><span class="t">{{mainLbl}}</span><span class="s">{{mainSub}}</span></span><span class="chev">›</span></span></button></div>'
lap = ('<div class="lap">' + TOP + STEPS + '<div style="min-height: 0; display: flex; flex-direction: column">' + ''.join(B) + '</div>' + FOOT
       + '<sc-if value="{{ended}}" hint-placeholder-val="{{ false }}"><div class="endc"><span class="lbl">Campagne validée</span><span class="ttl" style="font-size: 40px">Les Cendres de Valombre est jouable.</span><p class="nar" style="font-size: 22px">Le lien est parti à Camille, Marc et Hugo. Rendez-vous jeudi 19 septembre.</p>'
       + '<button type="button" class="go" style="width: 300px" onClick="{{restart}}"><span class="in"><span style="flex: 1"><span class="t">Rejouer la préparation</span></span><span class="chev">›</span></span></button></div></sc-if></div>')

SIDE = '''<div class="side">
<span class="lbl">Dans la tête de Romain, le MJ</span><div class="think">{{thought}}</div>
<span class="lbl">Pourquoi cet écran</span><div class="why">{{why}}</div>
<span class="lbl">La préparation · moment {{m}} sur 10</span><div class="prog"><sc-for list="{{dots}}" as="d" hint-placeholder-count="10"><button type="button" class="{{d.cls}}" onClick="{{d.pick}}">{{d.n}}</button></sc-for></div>
<div class="ctl"><button type="button" onClick="{{restart}}">Recommencer</button></div>
</div>'''

JS = r'''
class Component extends DCLogic {
  constructor(props) {
    super(props);
    this.state = this.fresh();
    this.timers = [];
    const pm = Number(props && props.moment);
    if (pm >= 1 && pm <= 10) Object.assign(this.state, this.settled(pm));
  }
  fresh() { return { m: 1, uni: 0, gen: 0, accepted: false, fixed: false, media: 0, sel: 'n4', ended: false }; }
  // A frozen moment (storyboard) shows the screen with the GM's decision still to take, except where the wait is the subject.
  settled(m) { return { m, gen: m === 5 ? 3 : m > 5 ? 6 : 0, accepted: m > 6, fixed: m > 7, media: m > 9 ? 2 : 0 }; }
  later(fn, ms) { this.timers.push(setTimeout(fn, ms)); }
  clear() { this.timers.forEach((t) => clearTimeout(t)); this.timers = []; }
  go(m) {
    if (m > 10) return;
    this.clear();
    const patch = { m, ended: false };
    // Jumping to a moment lands on it as if every earlier decision had been taken.
    patch.gen = m > 5 ? 6 : 0;
    patch.accepted = m > 6;
    patch.fixed = m > 7;
    patch.media = m > 9 ? 2 : 0;
    this.setState(patch);
    if (m === 5) for (let i = 1; i <= 6; i++) this.later(() => this.setState({ gen: i }), i * 1300);
  }
  renderVals() {
    const s = this.state, m = s.m;
    const G = [['La bible', 'le monde, son ton', 0.18], ['Les fronts', '2 menaces, leurs horloges', 0.10], ['Les scènes', '8 scènes, 3 actes', 0.46], ['Les fiches', '5 PNJ, 3 monstres, 6 objets', 0.30], ['Les indices', '3 révélations', 0.08], ['La cohérence', 'règle des 3 indices', 0.04]];
    const genCost = G.slice(0, s.gen).reduce((a, g) => a + g[2], 0);
    let spent = m >= 5 ? genCost : 0;
    if (s.accepted) spent += 0.03;
    if (s.fixed) spent += 0.02;
    if (s.media >= 2) spent += 0.32;
    const money = (x) => x.toFixed(2).replace('.', ',') + ' $';
    const N = [
      { id: 'n1', x: 12, y: 24, kind: 'Lieu', act: 'Acte I', title: 'L’auberge du Pendu', meta: '3 PNJ · 2 indices', goal: 'Accrocher le groupe à la ville.', read: 'La pluie tambourine sur les volets. Au comptoir, personne ne parle des enterrements de la nuit.', clues: [{ name: 'Pelle couverte de cendre', to: 'Le rituel approche' }], map: 'lieu · H-4' },
      { id: 'n2', x: 12, y: 128, kind: 'Scène', act: 'Acte I', title: 'Le cimetière sous la pluie', meta: '1 PNJ · 2 indices', goal: 'Montrer que les tombes sont vides.', read: 'La terre est fraîche sur une tombe vieille de vingt ans. Quelqu’un a creusé, puis tout remis en place.', clues: [{ name: 'Empreintes de petites bottes', to: 'Les gobelins travaillent pour quelqu’un' }], map: 'lieu · I-6' },
      { id: 'n3', x: 12, y: 232, kind: 'Situation', act: 'Acte I', title: 'La place du marché', meta: '2 PNJ · 1 indice', goal: 'Faire rencontrer le maire.', read: 'Le maire Aldric serre des mains. Son sourire s’arrête net quand on prononce le mot « crypte ».', clues: [{ name: 'Bague au sceau des Valombre', to: 'Le maire paie le culte' }], map: 'lieu · H-5' },
      { id: 'n7', x: 12, y: 336, kind: 'Lieu', act: 'Acte I', title: 'La mine d’argent', meta: '1 PNJ · 1 indice', goal: 'Montrer d’où vient l’argent neuf.', read: 'Les wagonnets sont vides, et pourtant la paie tombe chaque semaine.', clues: [{ name: 'Lingots sans poinçon', to: 'Le maire paie le culte' }], map: 'lieu · J-2' },
      { id: 'n4', x: 258, y: 64, kind: 'Donjon', act: 'Acte II', title: 'La crypte des Valombre', meta: '2 monstres · 3 indices · combat', goal: 'Découvrir qui protège le culte.', read: 'L’escalier descend plus loin que la colline ne le permet. En bas, l’air sent le fer et la cire froide.', clues: [{ name: 'Registre des sépultures', to: 'Le maire paie le culte' }, { name: 'La page arrachée', to: 'Le rituel approche' }], map: 'rencontre · crypte' },
      { id: 'n5', x: 258, y: 192, kind: 'Lieu', act: 'Acte II', title: 'Le manoir du maire', meta: '1 PNJ · 2 indices', goal: 'Confronter Aldric.', read: 'Des portraits partout, tous tournés vers la colline. Le bureau est fermé à clé.', clues: [{ name: 'Lettre non envoyée', to: 'Le maire paie le culte' }], map: 'lieu · G-3' },
      { id: 'n8', x: 258, y: 320, kind: 'Scène', act: 'Acte II', title: 'Le couvre-feu', meta: 'front · milice', goal: 'Mettre la pression.', read: 'La cloche sonne trois fois. La milice ferme les rues de la ville basse.', clues: [], map: 'lieu · ville' },
      { id: 'n6', x: 504, y: 180, kind: 'Climax', act: 'Acte III', title: 'Le rituel des cendres', meta: '1 PNJ · combat', goal: 'Empêcher le réveil des morts.', read: 'Cent tombes ouvertes, et au centre, une flamme qui ne réchauffe pas.', clues: [], map: 'rencontre · colline' }
    ];
    const byId = Object.fromEntries(N.map((n) => [n.id, n]));
    const links = [['n1', 'n4'], ['n2', 'n4'], ['n2', 'n5'], ['n3', 'n5'], ['n7', 'n8'], ['n4', 'n6'], ['n5', 'n6'], ['n8', 'n6']];
    const edges = [];
    links.forEach(([a, b], i) => {
      const A = byId[a], B = byId[b];
      const x1 = A.x + 180, y1 = A.y + 44, x2 = B.x, y2 = B.y + 44, mx = x1 + Math.round((x2 - x1) / 2) - (i % 3) * 6;
      const hot = A.id === s.sel || B.id === s.sel ? 'hot' : '';
      edges.push({ l: x1, t: y1 - 1, w: mx - x1, h: 2, cls: hot });
      edges.push({ l: mx - 1, t: Math.min(y1, y2) - 1, w: 2, h: Math.abs(y2 - y1) + 2, cls: hot });
      edges.push({ l: mx, t: y2 - 1, w: x2 - mx, h: 2, cls: hot });
    });
    const sel = byId[s.sel] || byId.n4;
    const selClues = sel.id === 'n4' && !s.accepted ? sel.clues.slice(1) : sel.clues;
    const uniNames = ['Fantasy', 'Zombies', 'Spatial'];
    const stepNames = ['Univers', 'Règles', 'Pitch', 'Génération', 'Atelier', 'Cohérence', 'Fiches', 'Cartes et médias', 'Validation'];
    const bn = {
      1: ['you', 'Deux campagnes. Tu en commences une nouvelle ?'],
      2: ['you', 'Choisis l’univers de la campagne'],
      3: ['you', 'Les règles : garde le préréglage ou adapte-le'],
      4: ['you', 'Écris ton idée, puis lance la génération'],
      5: s.gen < 6 ? ['wait', 'Le co-MJ écrit… tu peux lire ce qui est fini'] : ['you', 'C’est écrit : 1,16 $. Ouvre l’atelier pour relire'],
      6: s.accepted ? ['you', 'Changements appliqués. Fais vérifier la cohérence'] : ['you', 'Le co-MJ propose 2 changements : à toi de trancher'],
      7: s.fixed ? ['you', 'Les trois révélations ont leurs trois indices'] : ['warn', 'Une révélation n’a que 2 indices'],
      8: ['you', 'Relis les fiches : PNJ, monstres, objets'],
      9: s.media === 1 ? ['wait', 'Les illustrations se génèrent en arrière-plan…'] : s.media === 2 ? ['you', 'Les lieux de l’acte I et de la crypte sont prêts'] : ['you', '4 illustrations manquent pour les premières sessions'],
      10: ['you', 'Tout est relu. La campagne peut devenir jouable']
    }[m];
    const main = {
      1: ['Nouvelle campagne', 'Univers, règles, pitch : trois écrans', true, () => this.go(2)],
      2: ['Continuer avec ' + uniNames[s.uni], 'Décors, personnages et objets de l’univers', true, () => this.go(3)],
      3: ['Garder D&D 5e (SRD)', 'Modifiable plus tard dans l’onglet Règles', true, () => this.go(4)],
      4: ['Générer la campagne', '≈ 1,16 $ sur ton budget de 10 $', true, () => this.go(5)],
      5: s.gen < 6 ? ['Génération en cours…', G[Math.min(s.gen, 5)][0] + ' · ' + money(genCost) + ' dépensés', false, null] : ['Ouvrir l’atelier', 'Relire le graphe avec le co-MJ', true, () => this.go(6)],
      6: s.accepted ? ['Vérifier la cohérence', 'Le co-MJ relit tout le graphe · ≈ 0,01 $', true, () => this.go(7)] : ['Accepter les 2 changements', 'Un indice ajouté, une motivation changée', true, () => this.setState({ accepted: true })],
      7: s.fixed ? ['Passer aux fiches', '14 fiches écrites par le co-MJ', true, () => this.go(8)] : ['Ajouter l’indice proposé', 'Dans le manoir du maire · 0,02 $', true, () => this.setState({ fixed: true })],
      8: ['Passer aux cartes et médias', 'Les fiches restent modifiables', true, () => this.go(9)],
      9: s.media === 0 ? ['Générer 4 illustrations', '≈ 0,32 $ · en arrière-plan', true, () => { this.setState({ media: 1 }); this.later(() => this.setState({ media: 2 }), 2600); }]
        : s.media === 1 ? ['Génération… 2 sur 4', 'Tu peux continuer à relire', false, null] : ['Passer à la validation', 'Dernier coup d’œil', true, () => this.go(10)],
      10: ['Valider la campagne', 'Elle devient jouable et le lien d’invitation part', true, () => { this.clear(); this.setState({ ended: true }); }]
    }[m];
    const thoughts = {
      1: 'Kerlann est finie, l’hôpital attendra. Ce soir, je prépare quelque chose de nouveau pour Camille, Marc et Hugo.',
      2: 'Marc débute : de la fantasy, c’est ce qu’il connaît le mieux. Des nains, des gobelins, une crypte.',
      3: 'D&D, ils en ont tous entendu parler. Je garde, je verrai plus tard pour simplifier.',
      4: 'J’ai mon idée depuis des semaines : une ville qui enterre ses morts deux fois. Je l’écris comme je la raconterais.',
      5: s.gen < 6 ? 'Je lis la bible pendant qu’il écrit la suite. Je vois ce que ça coûte au fur et à mesure.' : 'Un euro et quelques. Je regarde ce qu’il a fait.',
      6: s.accepted ? 'Le maire qui paie pour que les morts restent sous terre : bien meilleur. Je fais vérifier.' : 'Le maire est trop méchant. Je lui ai demandé plus ambigu ; je lis la proposition avant d’accepter.',
      7: s.fixed ? 'Trois indices partout. Si Marc rate le registre, il y a encore l’ordre de paie.' : 'Une révélation n’a que deux indices : si mes joueurs en ratent un, ils restent bloqués.',
      8: 'Le chef gobelin, caché derrière la porte est. Je ne le sors que s’ils ouvrent cette porte.',
      9: s.media === 2 ? 'La crypte a sa carte, son image, sa musique. L’acte III, je verrai plus tard.' : 'Je ne génère que ce qui sert aux deux premières sessions. Le reste attendra.',
      10: 'Tout est relu. J’envoie le lien, on commence le 19.'
    }[m];
    const why = {
      1: 'Le MJ retrouve ses campagnes en un coup d’œil : celle en cours, celles terminées, les brouillons. Une seule action : en commencer une.',
      2: 'L’univers est un pack. Choisir ici évite de mélanger des tuiles de crypte avec des personnages de vaisseau.',
      3: 'Les règles sont des données : les cartes d’action des joueurs en découlent. Le MJ voit tout de suite ce que ses joueurs auront en main.',
      4: 'Le pitch est en langue naturelle. Le coût s’affiche avant de lancer : rien ne part sans que le MJ sache combien.',
      5: 'La génération prend quelques minutes. Le MJ suit les étapes et le coût, peut interrompre, et lit ce qui est déjà fini.',
      6: 'L’ancien atelier « Construction d’histoire » : le graphe au centre, la conversation à gauche, la scène à droite. L’IA propose un diff, le MJ l’accepte.',
      7: 'La règle des trois indices est vérifiée par la machine, pas par la mémoire du MJ. Une alerte, une proposition, un clic.',
      8: 'Chaque fiche montre ce qui servira en jeu : stats en gemmes, actions en cartes, ce qui est caché aux joueurs.',
      9: 'Les médias coûtent cher : le MJ les lance lot par lot, lieu par lieu, avec le prix avant. Les cartes s’ouvrent dans l’éditeur.',
      10: 'Un récapitulatif, le lien d’invitation, la date. La campagne devient jouable : elle apparaît dans la liste des campagnes.'
    }[m];
    const media = [
      ['L’auberge du Pendu', 'proposée', 'ok', 'validée', 'ok', 'Taverne sous la pluie', '0,08 $'],
      ['Le cimetière sous la pluie', 'proposée', 'ok', s.media === 2 ? 'générée' : s.media === 1 ? 'en cours…' : 'à générer', s.media === 2 ? 'ok' : s.media === 1 ? 'pend' : 'todo', 'Cloches et pluie', '0,08 $'],
      ['La place du marché', 'proposée', 'ok', s.media === 2 ? 'générée' : s.media === 1 ? 'en cours…' : 'à générer', s.media === 2 ? 'ok' : s.media === 1 ? 'pend' : 'todo', 'Marché de village', '0,08 $'],
      ['La crypte des Valombre', 'retouchée', 'ok', s.media === 2 ? 'générée' : s.media === 1 ? 'en cours…' : 'à générer', s.media === 2 ? 'ok' : s.media === 1 ? 'pend' : 'todo', 'Ombres de la crypte', '0,08 $'],
      ['Le manoir du maire', 'proposée', 'ok', s.media === 2 ? 'générée' : 'à générer', s.media === 2 ? 'ok' : 'todo', '—', '0,08 $'],
      ['Le rituel des cendres', 'à faire', 'pend', 'plus tard', 'pend', '—', '—']
    ].map(([n, mm, mc, i, ic, y, c]) => ({ n, m: mm, mc, i, ic, y, c, cls: n === 'La crypte des Valombre' ? 'on' : '' }));
    const v = {
      m,
      campName: m === 1 ? 'Mes campagnes' : 'Les Cendres de Valombre', campState: m === 1 ? '2 campagnes' : 'brouillon',
      spentTxt: money(spent).replace(' $', ''), spentPct: Math.round(spent * 10),
      steps: stepNames.map((n, i) => ({ n, rail: i > 0, cls: i + 2 === m ? 'cur' : i + 2 < m ? 'done' : '' })),
      bnCls: bn[0], bnTxt: bn[1],
      mainLbl: main[0], mainSub: main[1], mainCls: main[2] ? '' : 'off', mainAct: () => { if (main[3]) main[3](); },
      newCls: 'on',
      gen: G.map(([n, d, c], i) => ({ n, d, c: i < s.gen ? money(c) : i === s.gen ? '…' : '', cls: i < s.gen ? 'done' : i === s.gen ? 'cur' : '' })),
      genNow: s.gen < 6 ? 'Le co-MJ écrit ' + G[s.gen][0].toLowerCase() + '…' : 'Fini. ' + money(genCost) + ' sur l’estimation de 1,16 $.',
      frontsOn: s.gen >= 2,
      costTxt: s.accepted ? 'appliqué' : 'claude-sonnet',
      diffCls: s.accepted ? 'ok' : '', diffTtl: s.accepted ? 'Appliqué · 2 changements' : 'Proposition · 2 changements', diffOpen: !s.accepted,
      edges, nodes: N.map((n) => ({ ...n, cls: n.id === sel.id ? 'on' : '', pick: () => this.setState({ sel: n.id }) })),
      sel: { ...sel, clues: selClues },
      alertCls: s.fixed ? '' : 'warn', alertMark: s.fixed ? '✓' : '!', alertCount: s.fixed ? '3 sur 3' : '2 sur 3', notFixed: !s.fixed, fixed: s.fixed,
      media, crIll: s.media === 2 ? '[ILLUSTRATION · GÉNÉRÉE · À VALIDER]' : '[ILLUSTRATION · À GÉNÉRER]',
      ended: s.ended, thought: thoughts, why,
      dots: Array.from({ length: 10 }, (_, i) => ({ n: i + 1, cls: i + 1 === m ? 'cur' : i + 1 < m ? 'done' : '', pick: () => this.go(i + 1) })),
      restart: () => { this.clear(); this.setState(this.fresh()); }
    };
    UNIS_JS
    for (let i = 1; i <= 10; i++) v['is' + i] = m === i;
    const seul = String(this.props.seul ?? false) === 'true';
    v.full = !seul;
    v.rootStyle = seul ? 'width: 1440px; height: 900px; padding: 0' : 'width: 2120px; height: 1020px; padding: 60px 56px';
    return v;
  }
}
'''
JS = JS.replace('UNIS_JS', 'for (let i = 0; i < 3; i++) { v[\'u\' + i] = s.uni === i ? \'on\' : \'\'; v[\'pickU\' + i] = () => this.setState({ uni: i }); }')

html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Préparer la campagne</title>
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
<script type="text/x-dc" data-dc-script data-props='{{"moment":{{"editor":"int","default":1,"min":1,"max":10}},"seul":{{"editor":"boolean","default":false}},"$preview":{{"width":2120,"height":1020}}}}'>{JS}</script>
</body>
</html>
'''
out = 'docs/design/canvas/Preparer-MJ.dc.html'
open(out, 'w').write(html)
d = json.load(open('docs/design/canvas/canvas.json'))
d['boards'].setdefault('Preparer-MJ.dc.html', {"x": 0, "y": 1880, "w": 2120, "h": 1020, "title": "Préparer · la campagne de Romain", "is_interactive": True})  # keep a layout set on the canvas
if 'Preparer-MJ.dc.html' not in d['order']: d['order'].append('Preparer-MJ.dc.html')
json.dump(d, open('docs/design/canvas/canvas.json', 'w'), ensure_ascii=False, indent=2)

import sys as _s; _s.path.insert(0, 'docs/design')
from scope import scope, check
_out, _ren, _dyn = scope(open(out).read(), 'pp-', keep={'on', 'off', 'done', 'cur', 'you', 'wait', 'warn', 'ok', 'todo', 'pend', 'hot', 'gen'})
open(out, 'w').write(_out)
print(len(_out), 'unsafe:', check(_out, 'pp-', _dyn))
