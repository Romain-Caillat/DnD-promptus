"""Generate the playable invitation board: Romain invites his table and validates characters (Inviter-MJ.dc.html).

Seven moments, the GM laptop and Marc's phone side by side: the invite link, players arriving,
character creation, the GM reviewing a sheet with the co-GM's rule check, sending one back with
a note, tying backstories to the story graph, and setting the first session date.
Run from the repository root.
"""
import json

css = r'''
body{margin:0;background:#0A0A0A}
.pm{font-family:'Chakra Petch',system-ui,sans-serif;color:#F2F2F2;background:radial-gradient(120% 60% at 50% 0%,#1C1C1C 0,#0C0C0C 55%,#050505 100%);-webkit-font-smoothing:antialiased}
.ttl{font-family:'Cinzel',Georgia,serif;font-weight:800;letter-spacing:.02em}
.nar{margin:0;font-family:'Cormorant Garamond',Georgia,serif;font-style:italic;font-weight:500;line-height:1.35;color:#E5E5E5}
.lbl{font-size:10px;font-weight:600;letter-spacing:.2em;text-transform:uppercase;color:#8C8C8C}
.lap{position:relative;width:1440px;height:900px;flex:none;overflow:hidden;box-sizing:border-box;display:grid;grid-template-rows:52px 1fr 78px;background:radial-gradient(90% 70% at 50% 0%,#1A1A1A 0,#0C0C0C 60%,#050505 100%);border-radius:14px;box-shadow:0 0 0 2px #2C2C2C,0 30px 60px rgba(0,0,0,.7)}
.top{display:flex;align-items:center;gap:18px;padding:0 18px;border-bottom:1px solid #1F1F1F;font-size:12px;color:#A3A3A3;white-space:nowrap}
.top .sep{width:1px;height:20px;background:#2A2A2A;flex:none}
.tabs{display:flex;gap:20px;margin-left:24px}
.tabs > span{font-size:11px;font-weight:700;letter-spacing:.1em;text-transform:uppercase;color:#6E6E6E;padding:17px 0;border-bottom:2px solid transparent}
.tabs > span.on{color:#F2F2F2;border-bottom-color:#F2F2F2}
.cols{display:grid;grid-template-columns:300px 1fr 360px;gap:14px;padding:14px 18px 0;min-height:0}
.col{display:flex;flex-direction:column;gap:14px;min-height:0;overflow:hidden}
.panel{background:linear-gradient(180deg,#181818,#111);border:1px solid #2C2C2C;border-radius:14px;box-shadow:inset 0 1px 0 rgba(255,255,255,.06),0 12px 30px rgba(0,0,0,.45)}
.ph{display:flex;justify-content:space-between;align-items:baseline;padding:12px 14px 8px}
.seat{display:flex;align-items:center;gap:12px;height:72px;padding:0 12px;margin:0 10px 6px;border-radius:12px;background:#121212;border:1px solid transparent}
.seat.cur{border-color:#F2F2F2;background:#1C1C1C}
.seat .who{display:flex;flex-direction:column;gap:3px;flex:1;min-width:0}
.seat .who > b{font-size:14px}
.seat .who > small{font-size:11px;color:#8C8C8C;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.pf{display:inline-grid;place-items:end center;width:44px;height:56px;border-radius:9px;background:linear-gradient(180deg,#262626,#121212);border:2px solid #3A3A3A;box-shadow:0 3px 0 #000;box-sizing:border-box;flex:none;overflow:hidden}
.pf.none{background:transparent;border:2px dashed #3A3A3A;box-shadow:none}
.st{font-size:10px;font-weight:700;letter-spacing:.08em;text-transform:uppercase;padding:4px 7px;border-radius:6px;white-space:nowrap;flex:none}
.st.ok{background:#EDEDED;color:#0A0A0A}
.st.todo{background:#FFD60A;color:#0A0A0A}
.st.warn{background:#0A0A0A;color:#FF9F1C;border:1.5px solid #FF9F1C}
.st.wait{border:1.5px dashed #5E5E5E;color:#A3A3A3}
.st.off{color:#5A5A5A;border:1px solid #2C2C2C}
.stage{flex:1;min-height:0;display:flex;flex-direction:column;gap:14px;overflow:hidden}
.hd{display:flex;align-items:baseline;gap:14px}
.foot{display:flex;align-items:center;gap:14px;padding:0 18px 14px}
.bn{flex:1;display:flex;align-items:center;gap:10px;height:48px;padding:0 16px;border-radius:12px;font-weight:700;font-size:15px;box-sizing:border-box}
.bn.you{background:#161616;border:1px solid #2C2C2C;color:#F2F2F2}
.bn.wait{background:#0A0A0A;border:1.5px dashed #5E5E5E;color:#D4D4D4}
.bn.warn{background:#0A0A0A;border:1.5px solid #FF9F1C;color:#F2F2F2}
.go{font:inherit;width:460px;flex:none;display:block;box-sizing:border-box;border:0;padding:6px;border-radius:12px;background:#EDEDED;color:#0A0A0A;text-align:left;cursor:pointer;box-shadow:0 4px 0 #8A8A8A,0 7px 0 #5A5A5A,0 14px 24px rgba(0,0,0,.5);transform-origin:50% 100%;transition:transform .1s,box-shadow .1s}
.go .in{display:flex;align-items:center;gap:12px;border:1.5px solid #0A0A0A;border-radius:8px;padding:6px 14px;height:42px;box-sizing:border-box}
.go .t{display:block;font-family:'Cinzel',Georgia,serif;font-weight:800;font-size:17px;line-height:1.05}
.go .s{display:block;font-size:11px;font-weight:600;color:#5A5A5A;margin-top:2px}
.go .chev{margin-left:auto;font-family:'Cinzel',serif;font-weight:800;font-size:22px}
.go:active{transform:perspective(500px) rotateX(14deg) translateY(4px);box-shadow:0 1px 0 #8A8A8A,0 2px 0 #5A5A5A}
.go.off{background:#141414;color:#8C8C8C;box-shadow:0 0 0 1.5px #2C2C2C;cursor:default;pointer-events:none}
.go.off .in{border:1.5px dashed #3A3A3A}
.go.off .s{color:#6E6E6E}
.btn{font:inherit;font-family:'Cinzel',Georgia,serif;font-size:12px;font-weight:800;color:#F2F2F2;background:#141414;border:0;border-radius:8px;height:34px;padding:0 12px;display:inline-flex;align-items:center;justify-content:center;gap:6px;cursor:pointer;white-space:nowrap;outline:1.5px solid #5A5A5A;outline-offset:-4px;box-shadow:0 0 0 1.5px #3A3A3A,0 3px 0 #000;flex:none}
.sec{display:flex;flex-direction:column;gap:8px}
.box{display:flex;align-items:center;gap:14px;padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1px solid #333;font-size:14px}
.box > code{flex:1;font-family:'Chakra Petch',monospace;font-size:14px;color:#F2F2F2}
.edit{padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1px solid #333;font-size:14px;line-height:1.55;color:#D4D4D4}
.edit > .lbl{display:block;margin-bottom:8px}
.rl{display:grid;grid-template-columns:130px 1fr;gap:12px;align-items:baseline;font-size:13px;line-height:1.45;padding:9px 12px;border-radius:10px;background:#121212;color:#D4D4D4}
.qr{display:grid;grid-template-columns:repeat(21,6px);gap:0;padding:10px;background:#F2F2F2;border-radius:8px;flex:none}
.qr > i{width:6px;height:6px}
.qr > i.k{background:#0A0A0A}
.sheet{display:grid;grid-template-columns:190px 1fr;gap:20px}
.pst{display:grid;place-items:end center;height:220px;border-radius:14px;background:radial-gradient(60% 30% at 50% 92%,rgba(255,255,255,.1),transparent),#141414;border:1px solid #2C2C2C;padding-bottom:14px;box-sizing:border-box}
.gems{display:flex;gap:14px;align-items:flex-end}
.gem{display:flex;flex-direction:column;align-items:center;gap:4px;font-size:10px;font-weight:600;letter-spacing:.12em;text-transform:uppercase;color:#8C8C8C}
.abs{display:grid;grid-template-columns:repeat(6,1fr);gap:8px}
.ab{display:flex;flex-direction:column;align-items:center;gap:2px;padding:8px 0;border-radius:10px;background:#121212;border:1px solid #2C2C2C}
.ab > small{font-size:10px;font-weight:700;letter-spacing:.14em;color:#8C8C8C}
.ab > b{font-size:20px}
.ab.bad{border:1.5px solid #FF9F1C}
.ab.bad > b{color:#FF9F1C}
.ab.fix{border:1.5px solid #F2F2F2;background:#1C1C1C}
.hand{display:flex;gap:12px}
.chk{display:flex;align-items:flex-start;gap:10px;font-size:13px;padding:10px 12px;border-radius:10px;background:#121212;line-height:1.45}
.chk > i{width:18px;height:18px;border-radius:5px;background:#EDEDED;color:#0A0A0A;display:grid;place-items:center;font-style:normal;font-weight:800;font-size:11px;flex:none;margin-top:1px}
.chk.warn{border:1.5px solid #FF9F1C}
.chk.warn > i{background:#FF9F1C}
.co{display:flex;flex-direction:column;gap:8px;padding:0 14px 14px}
.note{padding:12px 14px;border-radius:12px;background:#EDEDED;color:#0A0A0A;font-size:13px;line-height:1.5;box-shadow:0 3px 0 #8A8A8A}
.note > .lbl{display:block;color:#5A5A5A;margin-bottom:4px}
.hook{display:flex;flex-direction:column;gap:4px;padding:10px 12px;border-radius:10px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.45;color:#D4D4D4}
.hook > small{font-size:11px;color:#8C8C8C}
.hook.on{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.hook.on > small{color:#5A5A5A}
.days{display:flex;gap:10px}
.day{display:flex;flex-direction:column;align-items:center;gap:2px;width:96px;padding:12px 0;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:12px;color:#A3A3A3}
.day > b{font-family:'Cinzel',serif;font-size:26px;color:#F2F2F2}
.day.on{background:#EDEDED;border-color:#0A0A0A;color:#5A5A5A;box-shadow:0 3px 0 #8A8A8A}
.day.on > b{color:#0A0A0A}
.ans{display:flex;align-items:center;gap:10px;font-size:13px;padding:8px 12px;border-radius:10px;background:#121212;color:#D4D4D4}
.ans > b{width:80px}
.endc{position:absolute;inset:0;z-index:70;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:16px;text-align:center;background:rgba(5,5,5,.94)}
.ph2{position:relative;width:390px;height:844px;border-radius:44px;overflow:hidden;border:10px solid #050505;box-shadow:0 0 0 2px #2C2C2C,0 30px 60px rgba(0,0,0,.7);background:radial-gradient(130% 70% at 50% 0%,#1A1A1A 0,#0C0C0C 60%,#050505 100%);box-sizing:border-box;display:flex;flex-direction:column;flex:none}
.pbn{margin:52px 12px 0;display:flex;align-items:center;gap:10px;padding:10px 14px;border-radius:12px;font-weight:700;font-size:14px}
.pbn.you{background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.pbn.wait{background:#0A0A0A;border:1.5px dashed #5E5E5E;color:#D4D4D4}
.pbn.listen{background:#161616;border:1px solid #2C2C2C;color:#D4D4D4}
.pbn.warn{background:#0A0A0A;border:1.5px solid #FF9F1C;color:#F2F2F2}
.pbody{flex:1;display:flex;flex-direction:column;gap:12px;padding:16px;min-height:0;overflow:hidden}
.pft{display:flex;flex-direction:column;gap:8px;padding:10px 14px 30px}
.cb{display:block;box-sizing:border-box;width:100%;padding:6px;border-radius:12px;background:#EDEDED;color:#0A0A0A;box-shadow:0 4px 0 #8A8A8A,0 7px 0 #5A5A5A}
.cb .in{display:flex;align-items:center;gap:12px;border:1.5px solid #0A0A0A;border-radius:8px;padding:8px 12px;min-height:40px}
.cb .t{display:block;font-family:'Cinzel',serif;font-weight:800;font-size:17px}
.cb .s{display:block;font-size:11px;font-weight:600;color:#5A5A5A;margin-top:2px}
.cb.dk{background:#141414;color:#F2F2F2;box-shadow:0 0 0 1.5px #3A3A3A,0 4px 0 #000}
.cb.dk .in{border-color:#5A5A5A}
.cb.dk .s{color:#8C8C8C}
.dm{display:flex;flex-direction:column;gap:10px;padding:12px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.5;color:#D4D4D4}
.dm .inv{display:flex;flex-direction:column;gap:4px;padding:12px;border-radius:10px;background:#EDEDED;color:#0A0A0A}
.field{display:flex;flex-direction:column;gap:6px}
.field > span:last-child{padding:12px 14px;border-radius:10px;background:#0D0D0D;border:1px solid #3A3A3A;font-size:16px}
.cls{display:grid;grid-template-columns:1fr 1fr;gap:8px}
.cl{display:flex;align-items:center;gap:10px;padding:10px;border-radius:12px;background:#141414;border:1.5px solid #2C2C2C;font-size:12px;color:#A3A3A3}
.cl > span > b{display:block;font-family:'Cinzel',serif;font-size:15px;color:#F2F2F2}
.cl.on{background:#EDEDED;border-color:#0A0A0A;color:#5A5A5A;box-shadow:0 3px 0 #8A8A8A}
.cl.on > span > b{color:#0A0A0A}
.dots{display:flex;gap:6px}
.dots > i{flex:1;height:4px;border-radius:2px;background:#2C2C2C}
.dots > i.on{background:#F2F2F2}
.pstage{display:flex;align-items:flex-end;gap:14px;padding:14px;border-radius:14px;background:radial-gradient(60% 30% at 30% 92%,rgba(255,255,255,.08),transparent),#141414;border:1px solid #2C2C2C}
.sent{display:flex;align-items:center;gap:10px;padding:12px 14px;border-radius:12px;background:#141414;border:1.5px dashed #5E5E5E;font-size:13px;color:#D4D4D4}
.spin{width:14px;height:14px;border-radius:50%;border:2px solid #3A3A3A;border-top-color:#F2F2F2;animation:spin 1s linear infinite;flex:none}
@keyframes spin{to{transform:rotate(360deg)}}
.toast{display:flex;flex-direction:column;gap:4px;padding:14px;border-radius:14px;background:#EDEDED;color:#0A0A0A;box-shadow:0 6px 0 #8A8A8A,0 20px 40px rgba(0,0,0,.6)}
.toast .lbl{color:#5A5A5A}
.side{display:flex;flex-direction:column;gap:18px;width:420px;flex:none}
.think{padding:14px 16px;border-radius:14px;background:#EDEDED;color:#0A0A0A;font-size:14px;line-height:1.45;box-shadow:0 3px 0 #8A8A8A;min-height:92px;box-sizing:border-box;display:flex;align-items:center}
.why{padding:12px 14px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.5;color:#D4D4D4;min-height:110px;box-sizing:border-box}
.prog{display:grid;grid-template-columns:repeat(7,1fr);gap:6px}
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


def when(n, body):
    return '<sc-if value="{{is%d}}" hint-placeholder-val="{{ %s }}">%s</sc-if>' % (n, 'true' if n == 1 else 'false', body)


def head(title, lbl):
    return f'<div class="hd"><span class="ttl" style="font-size: 26px">{title}</span><span class="lbl">{lbl}</span></div>'


def qr():
    cells = []
    for y in range(21):
        for x in range(21):
            finder = any(x0 <= x < x0 + 7 and y0 <= y < y0 + 7 and (x in (x0, x0 + 6) or y in (y0, y0 + 6) or (x0 + 2 <= x <= x0 + 4 and y0 + 2 <= y <= y0 + 4)) for x0, y0 in [(0, 0), (14, 0), (0, 14)])
            inside = any(x0 <= x < x0 + 8 and y0 <= y < y0 + 8 for x0, y0 in [(0, 0), (13, 0), (0, 13)])
            dark = finder or (not inside and ((x * 7 + y * 13 + x * y) % 5 < 2))
            cells.append('<i class="k"></i>' if dark else '<i></i>')
    return '<div class="qr">' + ''.join(cells) + '</div>'


def sheet(perso, name, line, gems, abilities, story, cards):
    g = ''.join(f'<span class="gem">{imp("Gemme", (40, 40), stat=s, valeur=v, taille=40, anim="false")}{n}</span>' for s, v, n in gems)
    return (f'<div class="sheet"><div class="pst">{imp("Perso", (100, 130), perso=perso, px=5, etat="aucun")}</div><div class="sec" style="gap: 12px">'
            f'<div class="hd"><span class="ttl" style="font-size: 26px">{name}</span><span class="lbl">{line}</span></div><div class="gems">{g}</div>'
            f'<div class="abs">{abilities}</div></div></div>'
            f'<div class="edit"><span class="lbl">Son histoire · écrite par le joueur</span>{story}</div>'
            '<div class="hand">' + ''.join(imp('GameCard', (100, 140), w=100, **c) for c in cards) + '</div>')


def abil(vals, bad=None, fix=None):
    out = ''
    for k, v in vals:
        cls = 'ab bad' if k == bad else 'ab fix' if k == fix else 'ab'
        out += f'<div class="{cls}"><small>{k}</small><b>{v}</b></div>'
    return out


LYRA = sheet('lyra', 'Lyra', 'elfe rôdeuse · niveau 1 · Camille',
             [('pv', '12', 'Vie'), ('ca', '15', 'Armure'), ('atq', '+5', 'Arc'), ('mvt', '7', 'Mouvement'), ('ini', '+3', 'Initiative')],
             abil([('FOR', 10), ('DEX', 17), ('CON', 12), ('INT', 10), ('SAG', 14), ('CHA', 10)]),
             'Lyra a grandi dans les bois au-dessus de Valombre. Elle descend en ville depuis que les oiseaux ont cessé de chanter près de la colline.',
             [dict(titre='Arc long', type='Arme', texte='1d8 + 3, de loin.', valeur='+5', stat='atq', icon='epee', rarete='commune'),
              dict(titre='Pistage', type='Sagesse', texte='Suivre une trace.', valeur='+4', stat='none', icon='oeil', rarete='commune'),
              dict(titre='Ennemi juré', type='Compétence', texte='Avantage contre les morts.', valeur='', stat='none', icon='crane', rarete='peu-commune')])
BORIN_CARDS = [dict(titre='Hache d’armes', type='Arme', texte='1d12 + 3 dégâts.', valeur='+5', stat='atq', icon='epee', rarete='commune'),
               dict(titre='Second souffle', type='Compétence', texte='Soigne 1d10 + 1.', valeur='d10', stat='pv', icon='sort', rarete='peu-commune'),
               dict(titre='Fouiller', type='Sagesse', texte='Chercher, observer.', valeur='+1', stat='none', icon='oeil', rarete='commune')]
BORIN_STORY = 'Borin a travaillé vingt ans dans la mine d’argent. Son frère Dorn y est descendu un soir et n’est jamais remonté. Borin veut savoir pourquoi la mine paie encore.'
BORIN_BAD = sheet('borin', 'Borin', 'nain guerrier · niveau 1 · Marc',
                  [('pv', '13', 'Vie'), ('ca', '18', 'Armure'), ('atq', '+6', 'Hache'), ('mvt', '5', 'Mouvement'), ('ini', '+0', 'Initiative')],
                  abil([('FOR', 18), ('DEX', 10), ('CON', 16), ('INT', 8), ('SAG', 12), ('CHA', 8)], bad='FOR'), BORIN_STORY, BORIN_CARDS)
BORIN_OK = sheet('borin', 'Borin', 'nain guerrier · niveau 1 · Marc',
                 [('pv', '13', 'Vie'), ('ca', '18', 'Armure'), ('atq', '+5', 'Hache'), ('mvt', '5', 'Mouvement'), ('ini', '+0', 'Initiative')],
                 abil([('FOR', 17), ('DEX', 10), ('CON', 16), ('INT', 8), ('SAG', 13), ('CHA', 8)], fix='SAG'), BORIN_STORY, BORIN_CARDS)

C = []
C.append(when(1, head('Inviter la table', 'Les Cendres de Valombre · 3 places')
  + '<div style="display: flex; gap: 18px; align-items: flex-start">' + qr() + '<div class="sec" style="flex: 1">'
  + '<span class="lbl">Le lien</span><div class="box"><code>Lien d’invitation · Les Cendres de Valombre</code><button type="button" class="btn">Nouveau lien</button></div>'
  + '<div class="rl"><span class="lbl">Valable</span><span>7 jours, 3 places. Le lien ne donne accès qu’à cette campagne.</span></div>'
  + '<div class="rl"><span class="lbl">Personnages</span><span>Créés par les joueurs avec les règles D&amp;D 5e (SRD), niveau 1. Tu valides chacun.</span></div></div></div>'
  + '<div class="edit"><span class="lbl">Le message à coller sur Discord · modifiable</span>Salut ! Nouvelle campagne : <b>Les Cendres de Valombre</b>, une ville minière qui enterre ses morts deux fois. Ouvrez le lien, choisissez un pseudo et créez votre personnage, je valide. Pas besoin de compte. Première session vers le 19 septembre.</div>'))
C.append(when(2, head('Ils arrivent', 'en direct')
  + '<div class="sec"><div class="ans"><b>Hugo</b><span>reprend Sef, son roublard du Phare de Kerlann. Validé hier.</span></div>'
  + '<div class="ans"><b>Camille</b><span>{{cTxt}}</span></div><div class="ans"><b>Marc</b><span>{{mTxt}}</span></div></div>'
  + '<div class="rl"><span class="lbl">Ce qu’ils voient</span><span>Le nom de la campagne, ton pitch, un pseudo à choisir, puis le créateur de personnage. Ni compte, ni mot de passe : leur appareil garde une clé secrète.</span></div>'
  + '<div class="rl"><span class="lbl">Spectateur</span><span>Quelqu’un peut aussi rejoindre pour regarder seulement : il voit la TV, pas les fiches.</span></div>'))
C.append(when(3, LYRA))
C.append(when(4, BORIN_BAD))
C.append(when(5, '<sc-if value="{{notResent}}" hint-placeholder-val="{{ true }}">' + BORIN_BAD + '</sc-if><sc-if value="{{resent}}" hint-placeholder-val="{{ false }}">' + BORIN_OK + '</sc-if>'))
C.append(when(6, head('La table est complète', 'tisser leurs histoires dans la tienne')
  + '<div style="display: flex; gap: 14px">' + ''.join(f'<div class="pst" style="flex: 1; height: 190px; display: flex; flex-direction: column; align-items: center; justify-content: flex-end; gap: 6px">{imp("Perso", (80, 104), perso=p, px=4, etat="aucun")}<span class="ttl" style="font-size: 16px">{n}</span><span style="font-size: 11px; color: #8C8C8C">{w}</span></div>' for p, n, w in [('lyra', 'Lyra', 'Camille · rôdeuse'), ('borin', 'Borin', 'Marc · guerrier'), ('sef', 'Sef', 'Hugo · roublard')]) + '</div>'
  + '<span class="lbl">Ce que leurs histoires apportent · le co-MJ propose</span>'
  + '<div class="hook {{hookCls}}"><b>Dorn, le frère de Borin</b><span>Il est le mort qui marche du rituel des cendres. Indice ajouté dans la mine : sa lampe, gravée à son nom.</span><small>→ nœuds La mine d’argent, Le rituel des cendres · gardé secret</small></div>'
  + '<div class="hook {{hookCls}}"><b>Les oiseaux de Lyra</b><span>Ils fuient la colline à chaque étape du culte. Lyra le remarque avant les autres.</span><small>→ front Le culte des cendres · un signe par étape</small></div>'))
C.append(when(7, head('La première session', 'proposée à la table')
  + '<div class="days">' + ''.join(f'<div class="day {"on" if d == "19" else ""}"><span>{j}</span><b>{d}</b><span>sept.</span></div>' for j, d in [('mar.', '17'), ('mer.', '18'), ('jeu.', '19'), ('ven.', '20'), ('sam.', '21')]) + '</div>'
  + '<div class="rl"><span class="lbl">Heure</span><span>20 h 30 · environ 2 h 30</span></div>'
  + '<div class="sec"><span class="lbl">Disponibilités</span><div class="ans"><b>Camille</b><span>jeudi ✓</span></div><div class="ans"><b>Marc</b><span>jeudi ✓</span></div><div class="ans"><b>Hugo</b><span>jeudi ✓ · vendredi ✓</span></div></div>'
  + '<div class="rl"><span class="lbl">Rappels</span><span>La veille et une heure avant, sur leurs téléphones. Chacun peut ajouter la date à son agenda.</span></div>'))

RIGHT = {
    1: '<div class="co"><div class="note"><span class="lbl">Le co-MJ</span>Le message reprend ton pitch sans rien dévoiler du maire ni de la crypte.</div></div>',
    2: '<div class="co"><div class="note"><span class="lbl">Le co-MJ</span>Il vérifiera chaque personnage contre les règles de la campagne dès qu’il arrive.</div></div>',
    3: '<div class="co"><div class="chk"><i>✓</i><span>Répartition des caractéristiques : 27 points, bonus d’elfe compris</span></div><div class="chk"><i>✓</i><span>Équipement de départ de la rôdeuse</span></div><div class="chk"><i>✓</i><span>Ses PV : 10 + Constitution</span></div><div class="note"><span class="lbl">Le co-MJ</span>Rien à redire. Son histoire touche la colline : je peux la relier au culte une fois la table complète.</div></div>',
    4: '<div class="co"><div class="chk warn"><i>!</i><span><b>Force 18 au niveau 1.</b> La répartition s’arrête à 15, 17 avec le bonus de nain.</span></div><div class="chk"><i>✓</i><span>Équipement de départ du guerrier</span></div><div class="chk"><i>✓</i><span>Ses PV : 10 + Constitution</span></div><div class="note"><span class="lbl">Mot proposé pour Marc · modifiable</span>Super Borin ! Juste un point : la Force s’arrête à 17 pour un nain au départ. Tu peux mettre le point en trop ailleurs ?</div></div>',
    5: '<div class="co"><sc-if value="{{notResent}}" hint-placeholder-val="{{ true }}"><div class="note"><span class="lbl">Envoyé à Marc</span>Super Borin ! Juste un point : la Force s’arrête à 17 pour un nain au départ. Tu peux mettre le point en trop ailleurs ?</div></sc-if><sc-if value="{{resent}}" hint-placeholder-val="{{ false }}"><div class="chk"><i>✓</i><span>Force 18 → 17, Sagesse 12 → 13 : répartition conforme</span></div><div class="chk"><i>✓</i><span>Rien d’autre n’a changé</span></div></sc-if></div>',
    6: '<div class="co"><div class="note"><span class="lbl">Le co-MJ</span>Deux accroches tirées de leurs histoires. Les joueurs n’en voient rien : elles vivent dans ton graphe.</div><div class="chk"><i>✓</i><span>Sef garde ce qu’il avait gagné au Phare : niveau 1 quand même, ses objets restent au coffre</span></div></div>',
    7: '<div class="co"><div class="note"><span class="lbl">Le co-MJ</span>Tout le monde peut jeudi. Je prépare le « Précédemment… » de la session 1 : une ouverture, puisqu’il n’y a rien avant.</div></div>',
}
RIGHT_HTML = ''.join(when(n, h) for n, h in RIGHT.items())

PHONE = {
    1: ('listen', 'Discord · Romain', '<div class="dm"><span><b>Romain</b> · aujourd’hui 18:04</span><span>Salut ! Nouvelle campagne : Les Cendres de Valombre, une ville minière qui enterre ses morts deux fois…</span><div class="inv"><span class="lbl" style="color: #5A5A5A">Promptus · invitation</span><b class="ttl" style="font-size: 17px">Les Cendres de Valombre</b><span style="font-size: 12px">Romain t’invite · 3 places</span></div></div>', ''),
    2: ('you', 'Tu es invité à une partie', '<span class="lbl">Romain t’invite</span><span class="ttl" style="font-size: 26px; line-height: 1.1">Les Cendres de Valombre</span><p class="nar" style="font-size: 18px">Une ville minière qui enterre ses morts deux fois.</p><div class="field"><span class="lbl">Ton pseudo</span><span>Marc</span></div><span style="font-size: 12px; color: #8C8C8C; line-height: 1.5">Pas de compte : ce téléphone garde ta place.</span>',
        '<div class="cb"><span class="in"><span style="flex: 1"><span class="t">Créer mon personnage</span><span class="s">Moins de deux minutes</span></span></span></div><div class="cb dk"><span class="in"><span style="flex: 1"><span class="t">Regarder seulement</span></span></span></div>'),
    3: ('you', 'Ton personnage · 2 sur 4', '<div class="dots"><i class="on"></i><i class="on"></i><i></i><i></i></div><span class="ttl" style="font-size: 22px">Ce que tu sais faire</span><div class="pstage">' + imp('Perso', (80, 104), perso='borin', px=4, etat='aucun') + '<div style="display: flex; flex-direction: column; gap: 4px"><span class="lbl">Nain</span><span class="ttl" style="font-size: 20px">Guerrier</span><span style="font-size: 12px; color: #A3A3A3">Solide, frappe fort, protège les autres.</span></div></div>'
        + '<div class="cls">' + ''.join(f'<div class="cl {"on" if n == "Guerrier" else ""}"><span><b>{n}</b>{d}</span></div>' for n, d in [('Guerrier', 'frapper, encaisser'), ('Rôdeur', 'tirer, pister'), ('Roublard', 'se cacher, crocheter'), ('Magicien', 'sorts, savoir')]) + '</div>',
        '<div class="cb"><span class="in"><span style="flex: 1"><span class="t">Continuer</span><span class="s">Tes caractéristiques</span></span></span></div>'),
    4: ('wait', 'Envoyé au MJ', '<div class="pstage">' + imp('Perso', (100, 130), perso='borin', px=5, etat='aucun') + '<div style="display: flex; flex-direction: column; gap: 4px"><span class="ttl" style="font-size: 22px">Borin</span><span style="font-size: 12px; color: #A3A3A3">nain guerrier · niveau 1</span></div></div><div class="sent"><span class="spin"></span>Romain regarde ton personnage…</div><span style="font-size: 12px; color: #8C8C8C; line-height: 1.5">Tu peux fermer l’appli : tu auras une notification quand il aura répondu.</span>', ''),
    5: ('warn', 'Le MJ te demande un changement', '<div class="note"><span class="lbl">Romain</span>Super Borin ! Juste un point : la Force s’arrête à 17 pour un nain au départ. Tu peux mettre le point en trop ailleurs ?</div><div class="abs" style="grid-template-columns: repeat(3, 1fr)">{{phoneAbs}}</div>',
        '<sc-if value="{{notResent}}" hint-placeholder-val="{{ true }}"><div class="cb"><span class="in"><span style="flex: 1"><span class="t">Renvoyer au MJ</span><span class="s">Force 17, Sagesse 13</span></span></span></div></sc-if><sc-if value="{{resent}}" hint-placeholder-val="{{ false }}"><div class="sent"><span class="spin"></span>Renvoyé. Romain regarde…</div></sc-if>'),
    6: ('you', 'Borin est validé !', '<div class="pstage">' + imp('Perso', (100, 130), perso='borin', px=5, etat='aucun') + '<div style="display: flex; flex-direction: column; gap: 4px"><span class="ttl" style="font-size: 22px">Borin</span><span style="font-size: 12px; color: #A3A3A3">nain guerrier · niveau 1</span><span class="lbl" style="color: #F2F2F2">✓ validé par Romain</span></div></div><span class="lbl">Ta table</span><div class="ans"><b>Lyra</b><span>Camille</span></div><div class="ans"><b>Sef</b><span>Hugo</span></div>', ''),
    7: ('listen', 'Les Cendres de Valombre', '<sc-if value="{{notEnded}}" hint-placeholder-val="{{ true }}"><div class="sent"><span class="spin"></span>Romain choisit la date de la première session…</div></sc-if><sc-if value="{{ended}}" hint-placeholder-val="{{ false }}"><div class="toast"><span class="lbl">Romain · session 1</span><b class="ttl" style="font-size: 20px">Jeudi 19 septembre · 20 h 30</b><span style="font-size: 12px">On commence à Valombre. Pense au son !</span></div></sc-if><div class="pstage">' + imp('Perso', (80, 104), perso='borin', px=4, etat='aucun') + '<span style="font-size: 13px; color: #D4D4D4; line-height: 1.5">Ta fiche est prête. Tu peux la relire avant jeudi.</span></div>',
        '<sc-if value="{{ended}}" hint-placeholder-val="{{ false }}"><div class="cb"><span class="in"><span style="flex: 1"><span class="t">Ajouter à mon agenda</span></span></span></div></sc-if>'),
}
phone = '<div class="ph2">' + ''.join(when(n, f'<div class="pbn {k}"><span>{b}</span></div><div class="pbody">{body}</div><div class="pft">{ft}</div>') for n, (k, b, body, ft) in PHONE.items()) + '</div>'

TOP = ('<header class="top"><span class="ttl" style="font-size: 13px; letter-spacing: .3em; color: #F2F2F2">PROMPTUS</span><span class="sep"></span>'
       '<span class="ttl" style="font-size: 15px; color: #F2F2F2">Les Cendres de Valombre</span><span class="tabs"><span>Histoire</span><span>Cartes</span><span>Médias</span><span class="on">Joueurs</span><span>Sessions</span></span><span style="flex: 1"></span><span>{{validCount}} sur 3 validés</span></header>')
LEFT = ('<div class="col"><section class="panel" style="flex: 1"><div class="ph"><span class="lbl" style="color: #F2F2F2">La table</span><span class="lbl">3 places</span></div>'
        '<sc-for list="{{seats}}" as="s" hint-placeholder-count="3"><div class="seat {{s.cls}}"><span class="pf {{s.pf}}"><sc-if value="{{s.has}}" hint-placeholder-val="{{ true }}"><dc-import name="Sprite" perso="{{s.p}}" px="2" anim="repos" hint-size="40px,52px"></dc-import></sc-if></span><div class="who"><b>{{s.n}}</b><small>{{s.c}}</small></div><span class="st {{s.st}}">{{s.stt}}</span></div></sc-for>'
        '<div style="padding: 6px 20px; font-size: 12px; color: #8C8C8C; line-height: 1.5">Un personnage refusé revient au joueur avec ton mot. Rien n’entre en jeu sans ta validation.</div></section></div>')
CENTER = '<div class="col"><div class="stage">' + ''.join(C) + '</div></div>'
RIGHTCOL = '<div class="col"><section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">{{rightLbl}}</span><span class="lbl">co-MJ</span></div>' + RIGHT_HTML + '</section></div>'
FOOT = '<div class="foot"><div class="bn {{bnCls}}"><span>{{bnTxt}}</span></div><button type="button" class="go {{mainCls}}" onClick="{{mainAct}}"><span class="in"><span style="flex: 1"><span class="t">{{mainLbl}}</span><span class="s">{{mainSub}}</span></span><span class="chev">›</span></span></button></div>'
lap = ('<div class="lap">' + TOP + '<div class="cols">' + LEFT + CENTER + RIGHTCOL + '</div>' + FOOT
       + '<sc-if value="{{ended}}" hint-placeholder-val="{{ false }}"><div class="endc"><span class="lbl">La table est prête</span><span class="ttl" style="font-size: 40px">Lyra, Borin et Sef partent jeudi.</span><p class="nar" style="font-size: 22px">La date est sur leurs téléphones. Le salon ouvrira jeudi à 20 h 15.</p>'
       + '<button type="button" class="go" style="width: 300px" onClick="{{restart}}"><span class="in"><span style="flex: 1"><span class="t">Rejouer l’invitation</span></span><span class="chev">›</span></span></button></div></sc-if></div>')

SIDE = '''<div class="side">
<span class="lbl">Dans la tête de Romain, le MJ</span><div class="think">{{thought}}</div>
<span class="lbl">Pourquoi cet écran</span><div class="why">{{why}}</div>
<span class="lbl">L’invitation · moment {{m}} sur 7</span><div class="prog"><sc-for list="{{dots}}" as="d" hint-placeholder-count="7"><button type="button" class="{{d.cls}}" onClick="{{d.pick}}">{{d.n}}</button></sc-for></div>
<div class="ctl"><button type="button" onClick="{{restart}}">Recommencer</button></div>
</div>'''

JS = r'''
class Component extends DCLogic {
  constructor(props) {
    super(props);
    this.state = this.fresh();
    this.timers = [];
    const pm = Number(props && props.moment);
    if (pm >= 1 && pm <= 7) Object.assign(this.state, this.settled(pm));
  }
  fresh() { return { m: 1, arrive: 0, resent: false, hooks: false, ended: false }; }
  // A frozen moment (storyboard) shows the decision the GM is about to take.
  settled(m) { return { m, arrive: m >= 2 ? 2 : 0, resent: m >= 5, hooks: m > 6 }; }
  later(fn, ms) { this.timers.push(setTimeout(fn, ms)); }
  clear() { this.timers.forEach((t) => clearTimeout(t)); this.timers = []; }
  go(m) {
    if (m > 7) return;
    this.clear();
    this.setState({ m, ended: false, arrive: m > 2 ? 2 : 0, resent: m > 5, hooks: m > 6 });
    if (m === 2) { this.later(() => this.setState({ arrive: 1 }), 1500); this.later(() => this.setState({ arrive: 2 }), 3500); }
    if (m === 5) this.later(() => this.setState({ resent: true }), 3000);
  }
  renderVals() {
    const s = this.state, m = s.m;
    const lyraOk = m > 3, borinOk = m > 5;
    const seat = (p, n, c, has, st, stt, cur) => ({ p, n, c, has, pf: has ? '' : 'none', st, stt, cls: cur ? 'cur' : '' });
    const lyra = m === 1 ? seat('lyra', 'Camille', 'pas encore là', false, 'off', 'invitée')
      : m === 2 ? (s.arrive >= 2 ? seat('lyra', 'Camille', 'Lyra · elfe rôdeuse', true, 'todo', 'à relire') : seat('lyra', 'Camille', 'crée son personnage…', false, 'wait', 'en cours'))
      : seat('lyra', 'Camille', 'Lyra · elfe rôdeuse', true, lyraOk ? 'ok' : 'todo', lyraOk ? 'validée' : 'à relire', m === 3);
    const borin = m === 1 ? seat('borin', 'Marc', 'pas encore là', false, 'off', 'invité')
      : m === 2 ? seat('borin', 'Marc', s.arrive >= 1 ? 'a rejoint · crée son personnage…' : 'ouvre le lien…', false, 'wait', s.arrive >= 1 ? 'en cours' : 'arrive')
      : m === 3 ? seat('borin', 'Marc', 'crée son personnage…', false, 'wait', 'en cours')
      : m === 4 ? seat('borin', 'Marc', 'Borin · nain guerrier', true, 'warn', '1 point', true)
      : m === 5 ? seat('borin', 'Marc', s.resent ? 'Borin · corrigé' : 'corrige avec ton mot…', true, s.resent ? 'todo' : 'wait', s.resent ? 'à relire' : 'renvoyé', true)
      : seat('borin', 'Marc', 'Borin · nain guerrier', true, 'ok', 'validé');
    const sef = seat('sef', 'Hugo', 'Sef · repris du Phare de Kerlann', true, 'ok', 'validé');
    const bn = {
      1: ['you', 'Envoie le lien à ta table'],
      2: s.arrive >= 2 ? ['you', 'Camille a envoyé Lyra : relis-la'] : ['wait', 'Ils arrivent…'],
      3: ['you', 'Lyra est conforme aux règles'],
      4: ['warn', 'Borin a un point qui ne passe pas les règles'],
      5: s.resent ? ['you', 'Marc a corrigé Borin'] : ['wait', 'Marc corrige avec ton mot…'],
      6: s.hooks ? ['you', 'Leurs histoires sont tissées dans la campagne'] : ['you', 'Le co-MJ propose deux accroches tirées de leurs histoires'],
      7: ['you', 'Tout le monde peut jeudi 19']
    }[m];
    const main = {
      1: ['Copier le lien et le message', 'À coller sur votre serveur Discord', true, () => this.go(2)],
      2: s.arrive >= 2 ? ['Relire Lyra', 'Le co-MJ l’a déjà vérifiée', true, () => this.go(3)] : ['En attente des personnages', 'Ils arrivent au fur et à mesure', false, null],
      3: ['Valider Lyra', 'Camille reçoit une notification', true, () => this.go(4)],
      4: ['Renvoyer à Marc avec ce mot', 'Ou accepte quand même : tu as le dernier mot', true, () => this.go(5)],
      5: s.resent ? ['Valider Borin', 'La table est complète', true, () => this.go(6)] : ['En attente de Marc', 'Il voit ton mot sur son téléphone', false, null],
      6: s.hooks ? ['Fixer la première session', 'Une date pour toute la table', true, () => this.go(7)] : ['Ajouter les deux accroches', 'Secrètes, dans ton graphe', true, () => this.setState({ hooks: true })],
      7: ['Envoyer la date', 'Notification et rappels sur leurs téléphones', true, () => { this.clear(); this.setState({ ended: true }); }]
    }[m];
    const thoughts = {
      1: 'Hugo m’a déjà dit qu’il reprenait Sef. Je colle le lien dans notre salon Discord.',
      2: s.arrive >= 2 ? 'Camille a été rapide. Marc crée encore son nain.' : 'Je les vois arriver sans rien faire. Pas de compte à créer pour eux.',
      3: 'Lyra est propre, et son histoire touche la colline. Je valide.',
      4: 'Marc a mis 18 en Force. C’est sa première fois, il n’a pas vu la limite. Je lui renvoie un mot gentil.',
      5: s.resent ? 'Il a mis le point en Sagesse. Parfait pour fouiller.' : 'Il reçoit mon mot sur son téléphone. J’attends.',
      6: s.hooks ? 'Le frère de Borin dans le rituel… Marc ne va pas s’en remettre.' : 'Son frère disparu dans la mine, c’est un cadeau pour mon histoire.',
      7: 'Jeudi pour tout le monde. J’envoie, et on se retrouve dans le salon.'
    }[m];
    const why = {
      1: 'Un lien par campagne, un message prêt à coller : le MJ invite là où sa table parle déjà. Le lien ne donne accès qu’à cette campagne.',
      2: 'Le MJ voit sa table se remplir en direct. Le joueur n’a ni compte ni mot de passe : son appareil garde une clé, le serveur n’en garde que l’empreinte.',
      3: 'La fiche est celle que le joueur verra en jeu. Le co-MJ vérifie les règles ; le MJ n’a plus qu’à lire l’histoire.',
      4: 'Une règle non respectée n’est pas un refus sec : le co-MJ écrit un mot, le MJ le relit et peut aussi accepter quand même.',
      5: 'Le joueur corrige sur son téléphone et renvoie. Le MJ voit ce qui a changé, rien d’autre.',
      6: 'Les histoires des joueurs nourrissent la campagne : le co-MJ propose des accroches, le MJ les garde secrètes dans son graphe.',
      7: 'La date part sur tous les téléphones, avec des rappels. Le jour venu, le salon ouvre un quart d’heure avant.'
    }[m];
    const v = {
      m, seats: [lyra, borin, sef], validCount: 1 + (lyraOk ? 1 : 0) + (borinOk ? 1 : 0),
      cTxt: s.arrive >= 2 ? 'a créé Lyra, une elfe rôdeuse. Envoyée pour validation.' : 'a rejoint, crée son personnage…',
      mTxt: s.arrive >= 1 ? 'a rejoint sur son téléphone, choisit sa classe…' : 'ouvre le lien sur son téléphone…',
      rightLbl: { 1: 'Invitation', 2: 'Vérification', 3: 'Vérification · Lyra', 4: 'Vérification · Borin', 5: 'Vérification · Borin', 6: 'Accroches', 7: 'Session 1' }[m],
      resent: s.resent, notResent: !s.resent, hookCls: s.hooks ? 'on' : '',
      phoneAbs: '',
      bnCls: bn[0], bnTxt: bn[1],
      mainLbl: main[0], mainSub: main[1], mainCls: main[2] ? '' : 'off', mainAct: () => { if (main[3]) main[3](); },
      ended: s.ended, notEnded: !s.ended, thought: thoughts, why,
      dots: Array.from({ length: 7 }, (_, i) => ({ n: i + 1, cls: i + 1 === m ? 'cur' : i + 1 < m ? 'done' : '', pick: () => this.go(i + 1) })),
      restart: () => { this.clear(); this.setState(this.fresh()); }
    };
    for (let i = 1; i <= 7; i++) v['is' + i] = m === i;
    const seul = String(this.props.seul ?? false) === 'true';
    v.full = !seul;
    v.rootStyle = seul ? 'width: 1880px; height: 900px; padding: 0' : 'width: 2450px; height: 1020px; padding: 60px 56px';
    return v;
  }
}
'''

# Marc's six abilities on the phone, the moved point highlighted: static markup, two states.
PH_ABS_BAD = abil([('FOR', 18), ('DEX', 10), ('CON', 16), ('INT', 8), ('SAG', 12), ('CHA', 8)], bad='FOR')
PH_ABS_OK = abil([('FOR', 17), ('DEX', 10), ('CON', 16), ('INT', 8), ('SAG', 13), ('CHA', 8)], fix='SAG')
phone = phone.replace('{{phoneAbs}}', '<sc-if value="{{notResent}}" hint-placeholder-val="{{ true }}">' + PH_ABS_BAD + '</sc-if><sc-if value="{{resent}}" hint-placeholder-val="{{ false }}">' + PH_ABS_OK + '</sc-if>')

html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>Inviter la table</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700;800&amp;family=Chakra+Petch:wght@400;500;600;700&amp;family=Cormorant+Garamond:ital,wght@1,500;1,600&amp;display=swap">
<style>{css}</style>
</helmet>
<div class="pm" style="{{{{rootStyle}}}}; position: relative; overflow: hidden; display: flex; gap: 40px; box-sizing: border-box; align-items: flex-start">
{lap}
{phone}
<sc-if value="{{{{full}}}}" hint-placeholder-val="{{{{ true }}}}">{SIDE}</sc-if>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"moment":{{"editor":"int","default":1,"min":1,"max":7}},"seul":{{"editor":"boolean","default":false}},"$preview":{{"width":2450,"height":1020}}}}'>{JS}</script>
</body>
</html>
'''
out = 'docs/design/canvas/Inviter-MJ.dc.html'
open(out, 'w').write(html)
d = json.load(open('docs/design/canvas/canvas.json'))
d['boards'].setdefault('Inviter-MJ.dc.html', {"x": 0, "y": 11400, "w": 2450, "h": 1020, "title": "Inviter · la table de Romain", "is_interactive": True})  # keep a layout set on the canvas
if 'Inviter-MJ.dc.html' not in d['order']: d['order'].append('Inviter-MJ.dc.html')
json.dump(d, open('docs/design/canvas/canvas.json', 'w'), ensure_ascii=False, indent=2)

import sys as _s; _s.path.insert(0, 'docs/design')
from scope import scope, check
_out, _ren, _dyn = scope(open(out).read(), 'iv-', keep={'on', 'off', 'done', 'cur', 'you', 'wait', 'warn', 'ok', 'todo', 'none', 'listen'})
open(out, 'w').write(_out)
print(len(_out), 'unsafe:', check(_out, 'iv-', _dyn))
