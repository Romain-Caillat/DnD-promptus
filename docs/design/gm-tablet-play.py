"""Generate the GM tablet: Romain runs session 3 from the couch, a tablet in his hands (Tablette-MJ.dc.html).

Five moments on the tablet: the scene, a player's request answered with big buttons, the map
handled by finger (fog brush), a fight where the co-GM proposes and one tap validates, and the
co-GM drawer driven by voice. Same session as Mener-MJ, laid out for touch: a rail of large
targets instead of tabs, nothing smaller than a finger. Run from the repository root.
"""
import sys
sys.path.insert(0, 'docs/design')
from kit import imp, when, iff, head, note, chk, top, laptop, side, component, board

P = 48
css = r'''
.tab .go{width:520px}
.tab .go .in{height:58px}
.tab .go .t{font-size:19px}
.tab .bn{height:62px;font-size:16px}
.rail{display:flex;flex-direction:column;gap:8px}
.rail > span{display:flex;flex-direction:column;align-items:center;justify-content:center;gap:4px;height:72px;border-radius:14px;background:#121212;border:1px solid #2C2C2C;font-size:11px;font-weight:600;color:#8C8C8C}
.rail > span > b{font-family:'Cinzel',serif;font-size:22px;color:#A3A3A3}
.rail > span.cur{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.rail > span.cur > b{color:#0A0A0A}
.rail > span > i{position:absolute;margin:-40px 0 0 40px;width:10px;height:10px;border-radius:50%;background:#FFD60A}
.mapw{position:relative;isolation:isolate;width:672px;height:432px;overflow:hidden;border-radius:14px;border:1px solid #2C2C2C;flex:none;align-self:center}
.ov{position:absolute;inset:0;z-index:950;pointer-events:none}
.fog{position:absolute;background:rgba(5,5,5,.78);border-left:2px dashed #5E5E5E}
.finger{position:absolute;width:64px;height:64px;border-radius:50%;transform:translate(-50%,-50%);border:2px solid #F2F2F2;background:rgba(242,242,242,.14);box-shadow:0 0 0 3px #0A0A0A}
.trail{position:absolute;height:64px;border-radius:32px;transform:translateY(-50%);background:rgba(242,242,242,.1)}
.hint{position:absolute;left:12px;bottom:12px;display:flex;gap:8px}
.hint > span{font-size:12px;font-weight:600;padding:6px 10px;border-radius:8px;background:#0A0A0A;border:1px solid #3A3A3A;color:#D4D4D4}
.req{display:flex;flex-direction:column;gap:14px;padding:18px;border-radius:16px;background:#EDEDED;color:#0A0A0A;box-shadow:0 6px 0 #8A8A8A,0 20px 40px rgba(0,0,0,.6)}
.req > .lbl{color:#5A5A5A}
.req > q{font-family:'Cormorant Garamond',serif;font-style:italic;font-size:24px;line-height:1.3;quotes:none}
.dcs{display:grid;grid-template-columns:repeat(4,1fr);gap:10px}
.dcs > span{display:flex;flex-direction:column;align-items:center;justify-content:center;height:72px;border-radius:12px;background:#FFF;border:2px solid #0A0A0A;font-size:12px;font-weight:600;color:#5A5A5A}
.dcs > span > b{font-size:26px;color:#0A0A0A}
.dcs > span.on{background:#0A0A0A;color:#A3A3A3}
.dcs > span.on > b{color:#F2F2F2}
.yn{display:grid;grid-template-columns:1fr 1fr;gap:10px}
.yn > span{display:flex;align-items:center;justify-content:center;height:60px;border-radius:12px;border:2px dashed #8A8A8A;font-weight:700;font-size:15px;color:#3A3A3A}
.stat{display:flex;align-items:center;gap:12px;padding:12px 14px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:14px;color:#D4D4D4}
.stat > b{margin-left:auto;font-size:22px;color:#F2F2F2}
.init{display:flex;gap:8px}
.init > span{display:flex;align-items:center;gap:8px;height:60px;padding:0 12px 0 6px;border-radius:12px;background:#121212;border:1px solid #2C2C2C;font-size:13px;font-weight:700;color:#A3A3A3}
.init > span.now{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.prop{display:flex;flex-direction:column;gap:10px;padding:16px;border-radius:16px;background:#0D0D0D;border:1.5px solid #F2F2F2;font-size:15px;line-height:1.5;color:#D4D4D4}
.prop > b{font-family:'Cinzel',serif;font-size:20px;color:#F2F2F2}
.big3{display:grid;grid-template-columns:2fr 1fr 1fr;gap:10px}
.big3 > span{display:flex;align-items:center;justify-content:center;height:64px;border-radius:12px;background:#141414;border:1.5px solid #3A3A3A;font-weight:700;font-size:15px}
.big3 > span.on{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A;box-shadow:0 4px 0 #8A8A8A}
.drawer{position:absolute;right:0;top:56px;bottom:92px;width:440px;z-index:60;display:flex;flex-direction:column;gap:12px;padding:18px;box-sizing:border-box;background:#0E0E0E;border-left:1px solid #3A3A3A;box-shadow:-30px 0 60px rgba(0,0,0,.7);animation:slide .35s ease-out}
@keyframes slide{from{transform:translateX(100%)}to{transform:none}}
.msg{padding:12px 14px;border-radius:12px;font-size:14px;line-height:1.45}
.msg.me{background:#1C1C1C;border:1px solid #3A3A3A;color:#F2F2F2;align-self:flex-end;max-width:85%}
.msg.ai{background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.msg.ai > .lbl{display:block;color:#5A5A5A;margin-bottom:4px}
.mic{display:flex;align-items:center;gap:12px;margin-top:auto;padding:10px;border-radius:14px;background:#141414;border:1px solid #2C2C2C;font-size:14px;color:#A3A3A3}
.mic > b{width:56px;height:56px;border-radius:50%;display:grid;place-items:center;background:#FF4D5E;color:#0A0A0A;font-size:22px;flex:none;box-shadow:0 0 0 6px rgba(255,77,94,.2)}
.mic.idle > b{background:#2C2C2C;color:#A3A3A3;box-shadow:none}
.seatc{display:flex;align-items:center;gap:10px;height:64px;padding:0 10px;margin:0 10px 6px;border-radius:12px;background:#121212}
.seatc > .who{display:flex;flex-direction:column;gap:2px;flex:1}
.seatc > .who > b{font-size:14px}
.seatc > .who > small{font-size:11px;color:#8C8C8C}
.seatc.ask{border:1.5px solid #FFD60A}
'''

RAIL = [('Scène', '❖'), ('Table', '☰'), ('Carte', '▦'), ('Journal', '✎'), ('Co-MJ', '✦')]
CUR = {1: 'Scène', 2: 'Scène', 3: 'Carte', 4: 'Carte', 5: 'Co-MJ'}


def plan(combat='false'):
    return f'<div style="position: absolute; left: 0; top: -{P}px">' + imp('Plan', (672, 480), theme='crypte', carte='salle', pitch=P, vue='mj', grille='true', combat=combat, bx=4, by=6) + '</div>'


def mapw(overlay, combat='false'):
    return f'<div class="mapw">{plan(combat)}<div class="ov">{overlay}</div></div>'


C = []
C.append(when(1, head('La salle de l’autel', 'scène 3 sur 5 · révélée')
  + '<div class="art" style="height: 230px"><span class="tag">IMAGE · RÉVÉLÉE</span><p class="nar" style="font-size: 22px; max-width: 560px">Des braseros éteints depuis des siècles s’allument d’eux-mêmes. Sur l’autel, une page arrachée tremble sans vent.</p></div>'
  + '<div class="row2">' + note('À lire si on te demande', 'L’autel porte le sceau du culte de la Cendre. Sagesse 13 pour la page, 15 pour la porte secrète.')
  + '<div class="yt" style="flex: none; align-self: flex-start"><span class="thumb">YT</span><span>Ombres de la crypte</span><span class="eq"><i></i><i></i><i></i></span></div></div>'
  + '<span class="mut">Tout se touche : une scène, une carte, un bouton. Rien de plus petit qu’un doigt.</span>'))
C.append(when(2, '<div class="req"><span class="lbl">Lyra · Camille demande</span><q>Je fouille l’autel, je cherche ce qui fait trembler la page.</q>'
  + '<span class="lbl">Un jet de Sagesse (Perception), difficulté</span><div class="dcs">'
  + ''.join(f'<span class="{{{{dc{v}}}}}"><b>{v}</b>{n}</span>' for v, n in [(12, 'Facile'), (13, 'Moyen'), (16, 'Difficile'), (20, 'Héroïque')])
  + '</div><div class="yn"><span>Oui, sans jet</span><span>Non, rien ici</span></div></div>'
  + iff('asked', '<div class="sent"><span class="spin"></span>Camille lance le dé sur son téléphone…</div>')
  + iff('found', '<div class="stat">' + imp('De', (56, 56), de='d20', valeur=15, taille=56, anim='fixe') + '<span>Lyra : 15 + 3 = <b style="font-size: 16px">18</b> contre 13</span><b>Réussi</b></div>'
        '<div class="chk"><i>✓</i><span>La page arrachée part dans le journal de tous. La porte secrète (15) aussi : 18 la trouve. Tu révèles ?</span></div>')))
C.append(when(3, head('La carte', 'au doigt', 22) + mapw(
    '<span class="fog" style="left: {{fogX}}px; top: 0; right: 0; bottom: 0"></span>'
    + '<span class="trail" style="left: 340px; top: 170px; width: 150px"></span><span class="finger" style="left: 490px; top: 170px"></span>'
    + '<div class="hint"><span>Un doigt : peindre le brouillard</span><span>Deux doigts : déplacer</span><span>Pincer : zoomer</span></div>')
  + '<span class="mut">Romain efface le brouillard du doigt, là où Lyra ouvre la porte secrète. Ce qui est levé part sur les téléphones et la TV au relâcher.</span>'))
C.append(when(4, '<div class="init">' + ''.join(f'<span class="{"now" if n == "Gobelin 1" else ""}"><span class="pf" style="width: 36px; height: 46px">{imp("Sprite", (20, 26), perso=p, px=1, anim="repos")}</span>{n}</span>' for p, n in [('sef', 'Sef'), ('gobelin', 'Gobelin 1'), ('borin', 'Borin'), ('lyra', 'Lyra'), ('gobelin', 'Gobelin 2')]) + '</div>'
  + mapw('', combat='true')
  + iff('notHit', '<div class="prop"><span class="lbl">Le co-MJ propose</span><b>Le gobelin 1 frappe Borin au cimeterre</b><span>Attaque 14 + 4 = 18 contre CA 18 : touché, 6 dégâts. Borin passe à 9 / 31.</span>'
        '<div class="big3"><span class="on">Valider</span><span>Changer</span><span>Il fuit</span></div></div>', True)
  + iff('hit', '<div class="chk"><i>✓</i><span>Validé. Marc voit ses cœurs baisser, la TV montre le coup. Au tour de Borin.</span></div>')))
C.append(when(5, head('La salle de l’autel', 'combat · tour 3') + mapw('', combat='true')
  + '<span class="mut">Le tiroir du co-MJ glisse par-dessus la scène, sans la quitter. On lui parle comme à un assistant à côté de soi.</span>'))

DRAWER = iff('drawer', '<div class="drawer"><div class="hd"><span class="ttl" style="font-size: 20px">Le co-MJ</span><span class="lbl">à la voix</span></div>'
  + '<div class="msg me">{{said}}</div>'
  + iff('replied', '<div class="msg ai"><span class="lbl">Je propose</span>Le gobelin 2 lâche son arme et file vers le puits. Il atteint la margelle au tour suivant, sauf si quelqu’un l’arrête. Sef peut lui couper la route : 4 cases.</div>'
        '<div class="big3"><span class="on">Faire</span><span>Changer</span><span>Non</span></div>')
  + '<div class="mic {{micCls}}"><b>●</b><span>{{micTxt}}</span></div></div>')

LEFT = '<div class="rail">' + ''.join(f'<span class="{{{{r{i}}}}}"><b>{g}</b>{n}</span>' for i, (n, g) in enumerate(RAIL)) + '</div>'
SEATS = [('borin', 'Borin', 'Marc · {{borinHp}} / 31', ''), ('lyra', 'Lyra', 'Camille · 20 / 24', '{{lyraCls}}'), ('sef', 'Sef', 'Hugo · à distance', '')]
RIGHT = ('<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">La table</span><span class="lbl">3 sur 3</span></div>'
         + ''.join(f'<div class="seatc {c}"><span class="pf" style="width: 40px; height: 50px">{imp("Sprite", (20, 26), perso=p, px=1, anim="repos")}</span><div class="who"><b>{n}</b><small>{w}</small></div><span class="pres on"></span></div>' for p, n, w, c in SEATS)
         + '</section><section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Le co-MJ</span><span class="lbl">tiroir</span></div><div class="co"><div class="note"><span class="lbl">{{coLbl}}</span>{{coTxt}}</div></div></section>')
TOP = top('Session 3 · la crypte', [], '', '<span>Soirée · 1 h 52</span><span class="sep"></span><span>TV connectée</span>')
tab = laptop(TOP, LEFT, ''.join(C), RIGHT, cols='84px 1fr 300px', cls='tab', overlay=DRAWER)

M = lambda lbl, sub, act, on='true': f'=[{lbl!r}, {sub!r}, {on}, () => {{ {act} }}]'
ROWS = {
    1: dict(bn=['you', 'La scène est révélée'], main=M('Ouvrir la demande de Lyra', 'Une main levée', 'this.go(2)'),
            thought='Le canapé, la tablette sur les genoux. Je vois tout sans me lever.',
            why='Sur tablette, le MJ mène debout ou assis loin de la table. Le rail remplace les onglets : cinq cibles de la taille d’un doigt, toujours au même endroit.',
            coLbl='Le co-MJ', coTxt='Lyra lève la main. Les deux autres écoutent.'),
    2: dict(bn=['=s.found ? "you" : s.asked ? "wait" : "you"', '=s.found ? "Lyra a trouvé la page" : s.asked ? "Camille lance le dé" : "Lyra demande à fouiller l’autel"'],
            main='=s.found ? ["Révéler la page et la porte", "Journal de tous, brouillard levé", true, () => this.go(3)] : ["Demander Sagesse 13", "Moyen · Camille lance", !s.asked, () => this.ask()]',
            thought='=s.found ? "18. Elle trouve tout, la porte aussi." : "Moyen. Treize. Elle est douée, mais c’est caché."',
            why='Une demande arrive en carte, pas en notification perdue. Les difficultés sont quatre grosses touches : le MJ répond d’un pouce, sans clavier.',
            coLbl='Le co-MJ', coTxt='=s.found ? "La porte secrète demandait 15 : je la révèle aussi si tu valides." : "La page demande Sagesse 13, comme dans ta prépa."'),
    3: dict(bn=['you', 'La porte secrète est ouverte'], main=M('Lever le brouillard ici', 'La salle est, pour tous', 'this.setState({ fog: true }); this.later(() => this.go(4), 900)', '!s.fog'),
            thought='=s.fog ? "Et les gobelins qui attendaient derrière." : "Du doigt, juste la salle est. Pas le puits."',
            why='La carte se manie comme une photo : un doigt peint le brouillard, deux doigts déplacent, on pince pour zoomer. Rien ne part avant le relâcher.',
            coLbl='Le co-MJ', coTxt='Deux gobelins dans la salle est. Je prépare l’initiative ?'),
    4: dict(bn=['=s.hit ? "you" : "turn"', '=s.hit ? "Au tour de Borin" : "Tour du gobelin 1"'],
            main='=s.hit ? ["Ouvrir le co-MJ", "Le gobelin 2 a peur", true, () => this.go(5)] : ["Valider l’attaque", "6 dégâts à Borin", true, () => this.setState({ hit: true })]',
            thought='=s.hit ? "Marc va grogner. Le gobelin 2, lui, il tremble." : "18 contre 18, ça touche. D’accord."',
            why='En combat, le co-MJ propose le tour des monstres et le MJ valide d’un geste. « Changer » et « Il fuit » restent à portée de pouce, jamais cachés.',
            coLbl='Le co-MJ', coTxt='=s.hit ? "Le gobelin 2 est à 3 PV. Il pourrait fuir." : "Je propose, tu tranches."'),
    5: dict(bn=['=s.replied ? "you" : "wait"', '=s.replied ? "Le co-MJ propose une fuite" : "Romain parle au co-MJ"'],
            main='=s.done ? ["Fait", "Le gobelin file vers le puits", false, null] : ["Faire fuir le gobelin", "Sef peut lui couper la route", s.replied, () => this.setState({ done: true })]',
            thought='=s.done ? "Sef va adorer. Hugo, à toi." : "Je ne tape pas, je parle. Comme à un ami à côté."',
            why='Le co-MJ s’ouvre en tiroir par-dessus la scène et s’écoute à la voix : sur tablette, dicter va plus vite que taper. Sa proposition reste à valider.',
            coLbl='Le co-MJ', coTxt='=s.done ? "Fait. Hugo voit le gobelin courir sur sa carte." : "J’écoute."'),
}
JS = component(5, ROWS,
               fresh='{ asked: false, found: false, fog: false, hit: false, said: false, replied: false, done: false }',
               settled='{ asked: m >= 2, found: m >= 2, fog: m >= 4, hit: m >= 5, said: m >= 5, replied: m >= 5 }',
               go='if (m === 5) { this.later(() => this.setState({ said: true }), 700); this.later(() => this.setState({ replied: true }), 2400); }',
               pre='this.ask = () => { this.setState({ asked: true }); this.later(() => this.setState({ found: true }), 1800); };',
               vals=f'''asked: s.asked && !s.found, found: s.found, hit: s.hit, notHit: !s.hit, replied: s.replied, drawer: m === 5,
      said: s.said ? 'Le gobelin 2 a peur. Fais-le fuir, mais laisse une chance à Sef.' : '…',
      micCls: s.replied ? 'idle' : '', micTxt: s.replied ? 'Touche pour parler' : 'J’écoute…',
      fogX: m === 3 && !s.fog ? 336 : 672, borinHp: m >= 4 && s.hit || m === 5 ? '9' : '15', lyraCls: m === 2 && !s.found ? 'ask' : '',
      ...Object.fromEntries([12, 13, 16, 20].map((v) => ['dc' + v, m === 2 && v === 13 && s.asked ? 'on' : ''])),
      ...Object.fromEntries({[n for n, _ in RAIL]}.map((n, i) => ['r' + i, n === {CUR}[m] ? 'cur' : ''])),''',
               full=(1752, 960), seul=(1180, 820))
parts = tab + iff('full', side('Romain, le MJ', 'La tablette', 5), True)
board('Tablette-MJ', 'Mener sur tablette', 'tb-', parts, JS, 5, (1752, 960),
      {"x": 2600, "y": 17600, "w": 1752, "h": 960, "title": "Tablette · mener la session 3 du canapé", "is_interactive": True}, css, keep={'ask', 'idle', 'now'})
