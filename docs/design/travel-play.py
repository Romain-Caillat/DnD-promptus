"""Generate the overland journey: Valombre to the abbey of Morneval (Voyager-MJ.dc.html).

Seven moments, the GM laptop and Marc's phone side by side, the TV in the side panel: two routes
proposed, the table votes, the days pass in portions, an event, a group check, the night watch,
the arrival at a place. Run from the repository root.
"""
import math, sys
sys.path.insert(0, 'docs/design')
from kit import imp, when, iff, head, note, chk, cb, top, laptop, phone, side, component, board

css = r'''
.hexw{position:relative;flex:none;border-radius:12px;overflow:hidden;border:1px solid #2C2C2C;background:#0A0D10}
.hexw > svg{display:block}
.hexw > svg > polygon{stroke:#0A0D10;stroke-width:2}
.hexw > svg > text{font-size:14px;fill:rgba(255,255,255,.18);text-anchor:middle}
.rt{fill:none;stroke:#F2F2F2;stroke-width:3;stroke-dasharray:2 7;stroke-linecap:round;opacity:.35}
.rt.on{opacity:1;stroke-width:4;stroke-dasharray:0}
.rt.alt{stroke:#FFD60A}
.rt.done{stroke-dasharray:0;opacity:1;stroke:#FFD60A}
.tok{position:absolute;transform:translate(-50%,-88%);z-index:5;transition:left 1s,top 1s;filter:drop-shadow(0 3px 0 rgba(0,0,0,.6))}
.pin{position:absolute;transform:translate(-50%,-50%);font-size:11px;font-weight:700;letter-spacing:.06em;background:#0A0A0A;border:1px solid #3A3A3A;padding:3px 7px;border-radius:6px;white-space:nowrap;z-index:4}
.pin.dest{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.rlab{position:absolute;font-size:11px;font-weight:700;padding:3px 7px;border-radius:6px;background:#0A0A0A;border:1.5px solid #F2F2F2;white-space:nowrap;z-index:4}
.rlab.alt{border-color:#FFD60A;color:#FFD60A}
.parts{display:grid;grid-template-columns:repeat(3,1fr);gap:6px}
.part{display:flex;flex-direction:column;gap:2px;padding:8px 10px;border-radius:10px;background:#121212;border:1px solid #2C2C2C;font-size:11px;color:#8C8C8C}
.part > b{font-size:13px;color:#F2F2F2}
.part.done{opacity:.55}
.part.cur{background:#EDEDED;border-color:#0A0A0A;color:#5A5A5A;box-shadow:0 3px 0 #8A8A8A}
.part.cur > b{color:#0A0A0A}
.rat{display:flex;gap:4px}
.rat > i{width:14px;height:18px;border-radius:3px;background:#F2F2F2}
.rat > i.off{background:#2C2C2C}
.vote{display:flex;flex-direction:column;gap:6px;padding:12px;border-radius:12px;background:#141414;border:1.5px solid #2C2C2C;font-size:12px;color:#A3A3A3;line-height:1.4}
.vote > b{font-family:'Cinzel',serif;font-size:16px;color:#F2F2F2}
.vote.on{border-color:#F2F2F2;background:#1C1C1C}
.vote > .who{display:flex;gap:6px}
.vote > .who > span{font-size:11px;font-weight:700;padding:2px 6px;border-radius:5px;background:#EDEDED;color:#0A0A0A}
.evt{display:flex;flex-direction:column;gap:4px;padding:10px 12px;border-radius:10px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.45;color:#D4D4D4}
.evt > small{font-size:11px;color:#8C8C8C}
.evt.on{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.evt.on > small{color:#5A5A5A}
.rolls{display:grid;grid-template-columns:repeat(3,1fr);gap:10px}
.roll{display:flex;flex-direction:column;align-items:center;gap:6px;padding:12px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:12px;color:#A3A3A3}
.roll > b{font-family:'Cinzel',serif;font-size:16px;color:#F2F2F2}
.roll > strong{font-size:22px;color:#F2F2F2}
.roll.ko > strong{color:#FF9F1C}
.watch{display:grid;grid-template-columns:110px 1fr;gap:8px;align-items:center;font-size:13px;padding:8px 12px;border-radius:10px;background:#121212;color:#D4D4D4}
.watch.on{border:1.5px solid #FFD60A}
.tvh{position:absolute;inset:0;display:flex;flex-direction:column;gap:12px;padding:28px 36px}
'''

# Pointy-top hex grid, offset rows. Terrain by hand so the two routes read clearly.
COLS, ROWS, R = 14, 9, 26
HW = math.sqrt(3) * R
TERRAIN = [
    'ppppmmmmmmpppp',
    'pphhmmmmmmhppp',
    'pphhhmmmmhhppp',
    'ppphhhhhhhpppw',
    'pppppffffppppw',
    'ppppfffffffppw',
    'pppffffffffpww',
    'wwpppffffpppww',
    'wwwppppppppwww',
]
FILL = {'p': '#20251D', 'f': '#16301E', 'h': '#2E2A20', 'm': '#3A3A40', 'w': '#142230'}
MARK = {'f': '♣', 'm': '▲', 'h': '⌒', 'w': '≈', 'p': ''}


def center(c, r):
    return (HW * (c + 0.5 * (r % 2)) + HW / 2 + 4, R * 1.5 * r + R + 4)


W_MAP = round(HW * (COLS + 0.5) + 8)
H_MAP = round(R * 1.5 * (ROWS - 1) + 2 * R + 8)
VALOMBRE, MORNEVAL = (1, 5), (12, 3)
ROUTE_COL = [(1, 5), (2, 4), (3, 3), (4, 2), (5, 1), (6, 1), (7, 1), (8, 1), (9, 2), (10, 3), (11, 3), (12, 3)]
ROUTE_FOR = [(1, 5), (2, 5), (3, 5), (4, 5), (5, 5), (6, 5), (7, 5), (8, 5), (9, 4), (10, 4), (11, 3), (12, 3)]
# Where the party stands at each moment along the forest road (index into ROUTE_FOR).
STEP = {1: 0, 2: 0, 3: 3, 4: 5, 5: 7, 6: 8, 7: 11}


def hexmap(scale=1.0):
    polys = []
    for r in range(ROWS):
        for c in range(COLS):
            x, y = center(c, r)
            pts = ' '.join(f'{x + R * math.cos(math.radians(60 * i - 30)):.0f},{y + R * math.sin(math.radians(60 * i - 30)):.0f}' for i in range(6))
            t = TERRAIN[r][c]
            polys.append(f'<polygon points="{pts}" fill="{FILL[t]}"></polygon>')
            if MARK[t]:
                polys.append(f'<text x="{x:.0f}" y="{y + 5:.0f}">{MARK[t]}</text>')

    def path(route, cls):
        d = 'M' + ' L'.join(f'{center(c, r)[0]:.1f},{center(c, r)[1]:.1f}' for c, r in route)
        return f'<path d="{d}" class="{cls}"></path>'
    svg = (f'<svg viewBox="0 0 {W_MAP} {H_MAP}" width="{round(W_MAP * scale)}" height="{round(H_MAP * scale)}" xmlns="http://www.w3.org/2000/svg">'
           + ''.join(polys) + path(ROUTE_COL, 'rt alt {{rCol}}') + path(ROUTE_FOR, 'rt {{rFor}}') + '</svg>')
    px = lambda cr: tuple(round(v * scale) for v in center(*cr))
    (vx, vy), (mx, my) = px(VALOMBRE), px(MORNEVAL)
    pins = (f'<span class="pin" style="left: {vx}px; top: {vy + round(30 * scale)}px">Valombre</span>'
            f'<span class="pin dest" style="left: {mx}px; top: {my - round(34 * scale)}px">Abbaye de Morneval</span>')
    labels = ''
    if scale >= .9:
        cx, cy = px((6, 0)); fx, fy = px((6, 6))
        labels = (f'<span class="rlab alt" style="left: {cx - 60}px; top: {cy + 18}px">Le col · 2 jours · froid</span>'
                  f'<span class="rlab" style="left: {fx - 60}px; top: {fy + 10}px">La forêt de Brune · 3 jours · loups</span>')
    tok = f'<div class="tok" style="{{{{tok{int(scale * 100)}}}}}">' + imp('Sprite', (round(40 * scale) or 20, round(52 * scale) or 26), perso='borin', px=max(1, round(2 * scale)), anim='marche') + '</div>'
    return f'<div class="hexw" style="width: {round(W_MAP * scale)}px; height: {round(H_MAP * scale)}px">{svg}{pins}{labels}{tok}</div>'


def tok_js(scale):
    pts = {m: tuple(round(v * scale) for v in center(*ROUTE_FOR[i])) for m, i in STEP.items()}
    return '{' + ', '.join(f'{m}: "left: {x}px; top: {y}px"' for m, (x, y) in pts.items()) + '}[m]'


PARTS = lambda cur, day: '<div class="parts">' + ''.join(
    f'<div class="part {"cur" if i == cur else "done" if i < cur else ""}"><span>Jour {day}</span><b>{n}</b></div>' for i, n in enumerate(('Matin', 'Après-midi', 'Soir'))) + '</div>'

C = []
C.append(when(1, head('Le voyage', 'Valombre → abbaye de Morneval') + hexmap(1)
  + '<span class="mut">Le co-MJ propose deux routes tirées de la carte du monde et des fronts de la campagne. Les joueurs choisissent.</span>'))
C.append(when(2, head('La table vote', 'sur les téléphones') + hexmap(1)
  + '<div class="row2"><div class="vote" style="flex: 1"><b>Le col</b><span>2 jours, froid, rien pour s’abriter.</span><div class="who"><span>Hugo</span></div></div>'
  '<div class="vote on" style="flex: 1"><b>La forêt de Brune</b><span>3 jours, des loups, mais du gibier.</span><div class="who"><span>Marc</span><span>Camille</span></div></div></div>'))
C.append(when(3, head('Jour 1', 'la forêt de Brune') + hexmap(1) + PARTS(1, 1)
  + '<div class="rl"><span class="lbl">Chaque portion</span><span>Avance d’un ou deux hexagones selon le terrain. Une ration par jour et par personne. Le co-MJ tire un événement par portion ; tu gardes ou tu passes.</span></div>'))
C.append(when(4, head('Jour 2 · après-midi', 'un événement') + '<div class="row2">' + hexmap(.62)
  + '<div class="sec" style="flex: 1"><span class="lbl">Le co-MJ propose · table des événements de la forêt</span>'
  '<div class="evt {{evA}}"><b>Un colporteur blessé</b><span>Il fuit l’abbaye : « Les moines ne dorment plus. »</span><small>→ indice vers le front du culte</small></div>'
  '<div class="evt"><b>Des traces de loups énormes</b><span>Fraîches, elles suivent le groupe.</span><small>→ rencontre la nuit</small></div>'
  '<div class="evt"><b>Rien, la pluie</b><span>Une portion sans histoire.</span><small>→ on avance</small></div></div></div>'))
C.append(when(5, head('Ne pas se perdre', 'jet de groupe · Survie DD 12') + '<div class="rolls">'
  + ''.join(f'<div class="roll {"ko" if v < 12 else ""}">{imp("Perso", (60, 78), perso=p, px=3, etat="aucun")}<b>{n}</b><strong>{{{{r_{p}}}}}</strong><span>{t}</span></div>' for p, n, v, t in [('borin', 'Borin', 9, '+1 Sagesse'), ('lyra', 'Lyra', 19, '+5 rôdeuse'), ('sef', 'Sef', 14, '+2')])
  + '</div><div class="rl"><span class="lbl">Règle</span><span>Jet de groupe : la moitié au moins doit réussir. Lyra porte le groupe ; Borin aurait tourné en rond.</span></div>'))
C.append(when(6, head('La nuit · jour 2', 'tours de garde') + hexmap(.62)
  + '<div class="watch"><b>Début de nuit</b><span>Lyra</span></div><div class="watch on"><b>Milieu</b><span>Borin · les loups approchent (vision dans le noir)</span></div><div class="watch"><b>Fin de nuit</b><span>Sef</span></div>'))
C.append(when(7, head('L’abbaye de Morneval', 'arrivée · jour 3, aube') + hexmap(1)
  + '<span class="mut">Le groupe arrive. L’échelle change : du monde en hexagones au lieu (la carte de l’abbaye), puis à la rencontre.</span>'))

LEFT = ('<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Le groupe</span><span class="lbl">{{dayTxt}}</span></div><div class="co">'
        '<div class="ans"><b>Rations</b><div class="rat"><sc-for list="{{rat}}" as="r" hint-placeholder-count="9"><i class="{{r.c}}"></i></sc-for></div></div>'
        '<div class="ans"><b>Distance</b><span>{{distTxt}}</span></div><div class="ans"><b>Temps</b><span>{{timeTxt}}</span></div></div></section>'
        '<section class="panel" style="flex: 1"><div class="ph"><span class="lbl" style="color: #F2F2F2">Pour toi seul</span></div><div class="co"><div class="secret"><span class="lbl">Ce qui attend</span><span>{{secret}}</span></div></div></section>')
RIGHT = '<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Le co-MJ</span></div><div class="co"><div class="note"><span class="lbl">Propose</span>{{coTxt}}</div></div></section>'
TOP = top('Les Cendres de Valombre', ['Jeu', 'Carte du monde', 'Journal'], 'Carte du monde', '<span>Session 5 · en direct</span>')
lap = laptop(TOP, LEFT, ''.join(C), RIGHT, cols='280px 1fr 320px')

P = {
    1: ('listen', 'Le MJ ouvre la carte du monde', hexmap(.5) + '<p class="nar" style="font-size: 18px">« L’abbaye est à l’est. Deux chemins : le col, ou la forêt de Brune. »</p>', ''),
    2: ('you', 'Par où passer ?', hexmap(.5) + '<div class="vote"><b>Le col</b><span>2 jours · froid, aucun abri</span></div><div class="vote on"><b>La forêt de Brune</b><span>3 jours · des loups, du gibier</span></div>',
        iff('notVoted', cb('Voter pour la forêt', 'Camille a voté forêt aussi', act='vote'), True) + iff('voted', '<div class="sent"><span class="spin"></span>Voté. Romain attend Hugo…</div>')),
    3: ('listen', 'Jour 1 · après-midi', hexmap(.5) + PARTS(1, 1) + '<div class="ans"><b>Rations</b><span>8 sur 9 · une mangée ce matin</span></div>', ''),
    4: ('listen', 'Sur la route', '<div class="art" style="height: 160px"><span class="ttl" style="font-size: 18px">Un colporteur blessé</span><span class="tag">[ILLUSTRATION]</span></div><p class="nar" style="font-size: 18px">« Les moines de Morneval ne dorment plus. Ils chantent, toute la nuit. »</p>', ''),
    5: ('you', 'Jet de groupe : Survie', '<div style="display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center"><span style="font-size: 15px">Il te faut <b>12 ou plus</b></span>'
        + imp('De', (130, 130), de='d20', valeur=8, taille=130, anim='fixe') + '<span style="font-size: 28px; font-weight: 700">8 + 1 = 9</span><span style="font-size: 13px; color: #A3A3A3">Raté, mais c’est un jet de groupe : Lyra a fait 19.</span></div>', ''),
    6: ('turn', 'Ton tour de garde', '<p class="nar" style="font-size: 18px">« Au milieu de la nuit, des yeux brillent entre les arbres. Toi seul les vois. »</p><div class="watch on"><b>Toi</b><span>vision dans le noir</span></div>',
        cb('Réveiller les autres', 'Ou les laisser dormir', act='wake')),
    7: ('listen', 'L’abbaye de Morneval', '<div class="art" style="height: 200px"><span class="ttl" style="font-size: 18px">L’abbaye à l’aube</span><span class="tag">[ILLUSTRATION]</span></div><p class="nar" style="font-size: 18px">« Les cloches sonnent. Personne ne vient ouvrir. »</p>', ''),
}
ph = phone(P, nav={n: 'Carte' if n in (1, 2, 3) else 'Jeu' for n in P})

TVH = {1: 'Le voyage · deux routes', 2: 'Le vote · la forêt de Brune', 3: 'Jour 1 · la forêt', 4: 'Un colporteur blessé', 5: 'Jet de groupe · réussi', 6: 'La nuit · Borin veille', 7: 'L’abbaye de Morneval'}
tvc = '<div class="tvh"><span class="ttl" style="font-size: 30px">{{tvTitle}}</span>' + hexmap(1) + '</div>'

M = lambda lbl, sub, act, on='true': f'=[{lbl!r}, {sub!r}, {on}, () => {{ {act} }}]'
ROWS = {
    1: dict(bn=['you', 'Le groupe veut rejoindre l’abbaye'], main=M('Proposer les deux routes', 'Aux joueurs, avec la durée et le risque', 'this.go(2)'),
            thought='Ils veulent l’abbaye. Le col est plus court, la forêt plus intéressante.',
            why='Le voyage se joue sur la carte du monde, en hexagones. Le co-MJ prépare les routes ; c’est la table qui choisit par où passer.',
            secret='Le culte a des guetteurs au col. La forêt cache le colporteur et une meute.', coTxt='Deux routes. La forêt donne une chance de croiser le colporteur, qui porte un indice du culte.'),
    2: dict(bn=['=s.voted ? "you" : "wait"', '=s.voted ? "La forêt gagne, 2 contre 1" : "Ils votent…"'], main=M('Partir par la forêt', 'Trois jours, découpés en portions', 'this.go(3)', 's.voted'),
            thought='=s.voted ? "La forêt. Je savais que Lyra voudrait y aller." : "Hugo veut le col. Il est pressé."',
            why='Chacun vote sur son téléphone ; le MJ voit qui veut quoi et tranche. Une décision de groupe prend dix secondes, sans couper la parole à personne.',
            secret='Le culte a des guetteurs au col. La forêt cache le colporteur et une meute.', coTxt='Marc et Camille veulent la forêt. Hugo préfère le col, il le dira peut-être en jeu.'),
    3: dict(bn=['you', 'Jour 1 · la marche'], main=M('Portion suivante', 'Le co-MJ tire un événement', 'this.go(4)'),
            thought='Je lis un paysage, j’avance le pion. Ça va vite.',
            why='Le temps passe par portions de journée : matin, après-midi, soir. Les rations baissent toutes seules, le pion avance sur la carte de tous.',
            secret='Rien sur cette portion.', coTxt='Une portion calme. J’en propose une animée à la suivante.'),
    4: dict(bn=['you', 'Un événement sur la route'], main=M('Garder le colporteur', 'Une petite scène, puis on repart', 'this.go(5)'),
            thought='Le colporteur. Ils vont adorer « les moines ne dorment plus ».',
            why='Le co-MJ propose trois événements tirés de la table du terrain ; le MJ en garde un, ou aucun. Ce qui arrive aux joueurs passe toujours par lui.',
            secret='Le colporteur ment sur une chose : il a vendu de l’huile au culte.', coTxt='Le colporteur relie le voyage au front du culte. Les loups peuvent attendre la nuit.'),
    5: dict(bn=['you', 'Jet de groupe réussi'], main=M('Continuer', 'La nuit tombe', 'this.go(6)'),
            thought='Borin aurait tourné en rond. Heureusement que Lyra est là.',
            why='Un jet de groupe se lance sur chaque téléphone en même temps. Le serveur applique la règle (la moitié doit réussir) ; le MJ lit le résultat.',
            secret='Raté, ils auraient perdu une demi-journée et une ration.', coTxt='Réussi : 2 sur 3. Le groupe garde son avance.'),
    6: dict(bn=['you', 'La garde de nuit'], main=M('Lancer la rencontre', 'Les loups · carte de la clairière', 'this.go(7)'),
            thought='Borin voit les loups venir. Je lui laisse le choix de réveiller les autres.',
            why='Les tours de garde décident qui voit venir le danger. Le MJ s’adresse au seul joueur éveillé, sur son téléphone.',
            secret='Quatre loups. Ils fuient si l’un d’eux tombe.', coTxt='Je prépare la carte de la clairière, au cas où.'),
    7: dict(bn=['you', 'Arrivée à l’abbaye'], main=M('Entrer dans l’abbaye', 'Carte du lieu', 'this.end()'),
            thought='Trois jours en vingt minutes de jeu. On y est.',
            why='À l’arrivée, l’échelle change : du monde au lieu, puis à la rencontre. Le voyage a coûté du temps et des rations, pas une soirée.',
            secret='Les moines chantent pour tenir un mort dans sa tombe.', coTxt='Le front du culte avance d’une part : trois jours ont passé.'),
}
JS = component(7, ROWS,
               fresh='{ voted: false, evA: false }',
               settled='{ voted: m >= 2, evA: m >= 4 }',
               go='if (m === 2) this.later(() => this.setState({ voted: true }), 2600); if (m > 2) this.setState({ voted: true }); if (m >= 4) this.setState({ evA: true });',
               vals=f'''tok100: {tok_js(1)}, tok62: {tok_js(.62)}, tok50: {tok_js(.5)},
      rCol: m <= 2 ? (s.voted && m === 2 ? '' : 'on') : 'off', rFor: m <= 2 ? (s.voted ? 'on' : 'on') : 'done',
      rat: Array.from({{ length: 9 }}, (_, i) => ({{ c: i < [9, 9, 8, 6, 5, 4, 3][m - 1] ? '' : 'off' }})),
      dayTxt: ['départ', 'départ', 'jour 1', 'jour 2', 'jour 2', 'nuit 2', 'jour 3'][m - 1],
      distTxt: ['0 sur 11 hexagones', '0 sur 11', '3 sur 11', '5 sur 11', '7 sur 11', '8 sur 11', '11 sur 11'][m - 1],
      timeTxt: ['', '', 'Jour 1 · après-midi', 'Jour 2 · après-midi', 'Jour 2 · soir', 'Nuit du jour 2', 'Jour 3 · aube'][m - 1] || 'Aube, jour 1',
      evA: s.evA ? 'on' : '', voted: s.voted, notVoted: !s.voted,
      r_borin: '9', r_lyra: '19', r_sef: '14', tvTitle: {TVH}[m],
      vote: () => this.setState({{ voted: true }}), wake: () => this.go(7),''',
               full=(2490, 1020), seul=(1880, 900))
parts = lap + ph + iff('full', side('Romain, le MJ', 'Le voyage', 7, tv=tvc), True)
board('Voyager-MJ', 'Le voyage', 'vy-', parts, JS, 7, (2490, 1020),
      {"x": 0, "y": 15600, "w": 2490, "h": 1020, "title": "Voyager · de Valombre à Morneval", "is_interactive": True}, css, keep={'alt'})
print('map', W_MAP, H_MAP)
