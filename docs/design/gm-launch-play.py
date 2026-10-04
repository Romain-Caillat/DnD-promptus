"""Generate the session launch: Romain pairs the shared screen and opens session 4 (Lancer-MJ.dc.html).

Five moments, the GM laptop beside the TV: the TV shows a code, the GM pairs it (or shares a
window on Discord instead), checks what the TV may show, plays "Previously…" and the first scene.
Run from the repository root.
"""
import sys
sys.path.insert(0, 'docs/design')
from kit import imp, when, iff, head, note, chk, top, laptop, component, board

css = r'''
.code{display:flex;gap:10px}
.code > b{width:64px;height:80px;border-radius:12px;display:grid;place-items:center;font-family:'Chakra Petch',monospace;font-size:44px;background:#EDEDED;color:#0A0A0A;box-shadow:0 4px 0 #8A8A8A}
.code.sm > b{width:44px;height:56px;font-size:30px;border-radius:9px}
.code.sm > b.e{background:#0D0D0D;color:#5A5A5A;border:1.5px dashed #5A5A5A;box-shadow:none}
.qr{display:grid;grid-template-columns:repeat(21,7px);padding:10px;background:#F2F2F2;border-radius:8px;flex:none}
.qr > i{width:7px;height:7px}
.qr > i.k{background:#0A0A0A}
.tvc{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:22px;text-align:center}
.tvseats{display:flex;gap:28px;align-items:flex-end}
.tvseat{display:flex;flex-direction:column;align-items:center;gap:8px;font-size:15px;color:#A3A3A3}
.tvseat > b{font-family:'Cinzel',serif;font-size:20px;color:#F2F2F2}
.tvseat.off{opacity:.35}
.tvyt{position:absolute;right:18px;bottom:16px;display:flex;align-items:center;gap:10px;padding:6px 12px 6px 6px;border-radius:10px;background:rgba(10,10,10,.85);border:1px solid #2C2C2C;font-size:13px;color:#D4D4D4}
.tvyt > .thumb{width:96px;height:54px;border-radius:6px;background:repeating-linear-gradient(135deg,#3A3A3A 0 2px,#262626 2px 7px);display:grid;place-items:center;font-size:9px;font-weight:700;letter-spacing:.1em}
.prev{position:absolute;inset:0;display:grid;grid-template-columns:1fr 1fr}
.prev > .img{background:radial-gradient(70% 80% at 50% 100%,rgba(10,10,10,.9),transparent),repeating-linear-gradient(135deg,#3A3A3A 0 2px,#2A2A2A 2px 9px);position:relative}
.prev > .txt{display:flex;flex-direction:column;justify-content:center;gap:16px;padding:40px}
.ln{opacity:.25}
.ln.on{opacity:1}
.opt3{display:grid;grid-template-columns:1fr 1fr;gap:12px}
.way{display:flex;flex-direction:column;gap:8px;padding:16px;border-radius:14px;background:#141414;border:1.5px solid #2C2C2C;font-size:13px;line-height:1.45;color:#A3A3A3}
.way > b{font-family:'Cinzel',serif;font-size:18px;color:#F2F2F2}
.way.on{border-color:#F2F2F2;background:#1C1C1C}
.see{display:grid;grid-template-columns:1fr 90px 90px;gap:8px;align-items:center;font-size:13px;padding:8px 12px;border-radius:10px;background:#121212;color:#D4D4D4}
.see > span:not(:first-child){text-align:center;font-size:11px;font-weight:700;letter-spacing:.08em;text-transform:uppercase}
.yes{color:#F2F2F2}
.no{color:#5A5A5A}
'''


def qr():
    cells = []
    for y in range(21):
        for x in range(21):
            finder = any(x0 <= x < x0 + 7 and y0 <= y < y0 + 7 and (x in (x0, x0 + 6) or y in (y0, y0 + 6) or (x0 + 2 <= x <= x0 + 4 and y0 + 2 <= y <= y0 + 4)) for x0, y0 in [(0, 0), (14, 0), (0, 14)])
            inside = any(x0 <= x < x0 + 8 and y0 <= y < y0 + 8 for x0, y0 in [(0, 0), (13, 0), (0, 13)])
            dark = finder or (not inside and ((x * 5 + y * 11 + x * y) % 5 < 2))
            cells.append('<i class="k"></i>' if dark else '<i></i>')
    return '<div class="qr">' + ''.join(cells) + '</div>'


YT = '<div class="tvyt"><span class="thumb">YOUTUBE</span><span class="eq"><i></i><i></i><i></i></span><span>Valombre, la nuit · {{ytTxt}}</span></div>'
SEATS = '<div class="tvseats">' + ''.join(
    f'<div class="tvseat {{{{tv_{p}}}}}">{imp("Perso", (80, 104), perso=p, px=4, etat="aucun")}<b>{n}</b><span>{w}</span></div>' for p, n, w in [('borin', 'Borin', 'Marc'), ('lyra', 'Lyra', 'Camille'), ('sef', 'Sef', 'Hugo')]) + '</div>'

TV = {
    1: '<div class="tvc"><span class="ttl" style="font-size: 18px; letter-spacing: .3em">PROMPTUS · TV</span><div style="display: flex; gap: 40px; align-items: center">' + qr()
       + '<div style="display: flex; flex-direction: column; gap: 14px; align-items: flex-start"><span class="lbl" style="font-size: 13px">Code de la TV</span><div class="code"><b>K</b><b>7</b><b>Q</b><b>F</b></div><span style="font-size: 15px; color: #A3A3A3">Le MJ saisit ce code, ou scanne le QR.</span></div></div></div>',
    2: '<div class="tvc"><span class="lbl" style="font-size: 13px">Connecté à la table de Romain</span><span class="ttl" style="font-size: 44px">Les Cendres de Valombre</span><span style="font-size: 18px; color: #A3A3A3">Session 4 · la partie commence bientôt</span>' + SEATS + '</div>' + YT,
    3: '<div class="tvc"><span class="lbl" style="font-size: 13px">Connecté à la table de Romain</span><span class="ttl" style="font-size: 44px">Les Cendres de Valombre</span><span style="font-size: 18px; color: #A3A3A3">Session 4 · tout le monde est là</span>' + SEATS + '</div>' + YT,
    4: '<div class="prev"><div class="img"><span class="tag">[ILLUSTRATION · LA CRYPTE]</span></div><div class="txt"><span class="lbl" style="font-size: 13px">Précédemment…</span>'
       '<p class="nar ln on" style="font-size: 26px">Sous l’autel, Borin a trouvé la page arrachée du registre.</p><p class="nar ln {{l2}}" style="font-size: 26px">Les Valombre y gardaient « la porte nord ».</p>'
       '<p class="nar ln {{l3}}" style="font-size: 26px">Et quelqu’un avait laissé des cendres encore tièdes.</p></div></div>' + YT,
    5: '<div class="prev"><div class="img"><span class="tag">[ILLUSTRATION · LA PORTE NORD]</span></div><div class="txt"><span class="lbl" style="font-size: 13px">Session 4 · scène 1</span><span class="ttl" style="font-size: 38px">La porte nord</span>'
       '<p class="nar" style="font-size: 24px">Une porte de bronze, plus haute que la crypte elle-même. Sur le linteau, un corbeau.</p>' + SEATS.replace('Perso', 'Perso') + '</div></div>' + YT,
}
tv = '<div class="tv">' + ''.join(when(n, h) for n, h in TV.items()) + '</div>'

C = []
C.append(when(1, head('Lancer la session 4', 'jeudi 10 octobre · 20 h 15')
  + '<span class="lbl">L’écran partagé · facultatif</span><div class="opt3">'
  '<div class="way on"><b>Une TV dans la pièce</b><span>Ouvre promptus.app/tv sur la TV ou un vieil ordinateur branché dessus, puis saisis son code ici.</span>'
  '<div class="code sm"><b class="e">·</b><b class="e">·</b><b class="e">·</b><b class="e">·</b></div></div>'
  '<div class="way"><b>Partager une fenêtre</b><span>Tout le monde est à distance : ouvre la fenêtre TV et partage-la dans Discord. Même contenu, sans code.</span><button type="button" class="btn" style="align-self: flex-start">Ouvrir la fenêtre TV</button></div></div>'
  + '<div class="rl"><span class="lbl">Sans écran partagé</span><span>Chaque téléphone joue les grands moments lui-même. La TV n’est qu’un bonus.</span></div>'))
C.append(when(2, head('La TV est jumelée', 'Salon · TV')
  + '<div class="row2"><div class="code sm"><b>K</b><b>7</b><b>Q</b><b>F</b></div><div class="sec" style="flex: 1"><span style="font-size: 15px; font-weight: 700">Salon · TV</span><span class="mut">Jumelée à 20 h 16. Elle suit la partie sans rien toucher : c’est toi qui décides ce qu’elle montre.</span></div><button type="button" class="btn">Oublier cette TV</button></div>'
  + '<div class="rl"><span class="lbl">Le son</span><span>La musique joue sur la TV et sur chaque téléphone, muette tant que le joueur n’a pas touché « activer le son ». Le lecteur YouTube reste visible.</span></div>'
  + '<div class="rl"><span class="lbl">La prochaine fois</span><span>La TV se souvient de la table : elle se reconnecte toute seule à l’ouverture du salon.</span></div>'))
C.append(when(3, head('Ce que la TV peut montrer', 'vérifié avant chaque envoi')
  + '<div class="sec"><div class="see"><span class="lbl">Contenu</span><span class="lbl">TV</span><span class="lbl">Joueurs</span></div>'
  + ''.join(f'<div class="see"><span>{a}</span><span class="{"yes" if b else "no"}">{"oui" if b else "non"}</span><span class="{"yes" if c else "no"}">{"oui" if c else "non"}</span></div>' for a, b, c in [
      ('La carte, sans le brouillard levé', True, True), ('Le groupe, l’ordre du tour, les menaces visibles', True, True),
      ('Les grands moments : dés, coups, butin, niveaux', True, True), ('La fiche complète d’un joueur', False, True),
      ('Tes notes, les PV des monstres, les zones cachées', False, False), ('Les propositions du co-MJ', False, False)])
  + '</div><span class="mut">La TV ne montre que ce que tous les joueurs ont le droit de voir. Rien de secret ne quitte ton écran.</span>'))
C.append(when(4, head('Précédemment…', 'sur la TV et les téléphones')
  + '<div class="edit"><span class="lbl">Relu mardi · lu par toi à voix haute, ou par la voix du co-MJ</span>'
  '<span class="ln on">Sous l’autel, Borin a trouvé la page arrachée du registre. </span><span class="ln {{l2}}">Les Valombre y gardaient « la porte nord ». </span><span class="ln {{l3}}">Et quelqu’un avait laissé des cendres encore tièdes.</span></div>'
  + '<div class="secret"><span class="lbl">Pour toi seul</span><span>Ce soir : la porte nord s’ouvre si Lyra suit les oiseaux. Le culte des cendres est à 3 parts sur 6.</span></div>'))
C.append(when(5, head('La porte nord', 'scène 1 · envoyée')
  + '<div class="art"><span class="ttl" style="font-size: 20px">La porte nord</span><span class="tag">[ILLUSTRATION]</span></div>'
  + '<span class="mut">La suite se joue comme la soirée de Marc : planche « Mener », moment 3.</span>'))

LEFT = ('<section class="panel" style="flex: 1"><div class="ph"><span class="lbl" style="color: #F2F2F2">Le salon</span><span class="lbl">{{presTxt}}</span></div>'
        '<sc-for list="{{seats}}" as="s" hint-placeholder-count="3"><div class="seat"><span class="pf"><dc-import name="Sprite" perso="{{s.p}}" px="2" anim="repos" hint-size="40px,52px"></dc-import></span><div class="who"><b>{{s.n}}</b><small>{{s.c}}</small></div><span class="pres {{s.on}}"></span></div></sc-for>'
        '<div class="seat"><span class="pf none"></span><div class="who"><b>Salon · TV</b><small>{{tvTxt}}</small></div><span class="pres {{tvOn}}"></span></div></section>')
RIGHT = ('<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Avant de lancer</span><span class="lbl">co-MJ</span></div><div class="co">'
         + chk('Le récap de la session 3 est publié depuis vendredi') + chk('Le « Précédemment… » est relu')
         + '<div class="chk {{tvChk}}"><i>{{tvMark}}</i><span>Écran partagé {{tvState}}</span></div>'
         + '<div class="chk {{allChk}}"><i>{{allMark}}</i><span>{{allTxt}}</span></div>'
         + '<div class="note"><span class="lbl">Le co-MJ</span>{{coTxt}}</div></div></section>')
TOP = top('Les Cendres de Valombre', ['Histoire', 'Cartes', 'Médias', 'Joueurs', 'Sessions'], 'Sessions', '<span>Session 4 · jeudi 10 octobre</span>')
lap = laptop(TOP, LEFT, ''.join(C), RIGHT)

M = lambda lbl, sub, act, on='true': f'=[{lbl!r}, {sub!r}, {on}, () => {{ {act} }}]'
ROWS = {
    1: dict(bn=['you', 'La TV du salon affiche un code'], main=M('Jumeler la TV', 'Code K7QF', 'this.go(2)'),
            thought='Ce soir Marc et Camille viennent chez moi, Hugo joue depuis Lyon. On met la TV du salon.',
            why='L’écran partagé est facultatif : une TV jumelée par code, ou une fenêtre partagée sur Discord quand tout le monde est à distance.'),
    2: dict(bn=['=s.all ? "you" : "wait"', '=s.all ? "Tout le monde est là" : "Hugo arrive…"'], main=M('Vérifier ce que voit la TV', 'Une fois, pour être tranquille', 'this.go(3)'),
            thought='=s.all ? "Hugo est arrivé, son micro marche. On peut y aller." : "La TV a trouvé la table toute seule. Il manque Hugo."',
            why='Jumeler prend quatre caractères. La TV montre le salon en attendant : qui est là, la musique. Elle ne reçoit jamais rien de secret.'),
    3: dict(bn=['you', 'Tout le monde est là'], main=M('Lancer la session', 'Le « Précédemment… » démarre partout', 'this.go(4)'),
            thought='Personne ne verra mes notes sur la TV. Parfait.',
            why='Le MJ sait ce qui peut partir sur la TV : ce que tous les joueurs ont le droit de voir, jamais ses notes ni les propositions du co-MJ.'),
    4: dict(bn=['you', 'Précédemment… à l’écran'], main=M('Envoyer la première scène', 'La porte nord', 'this.go(5)'),
            thought='Je lis le résumé moi-même, ça met dans l’ambiance.',
            why='Le « Précédemment… » relu avant la soirée passe sur la TV et les téléphones, phrase par phrase, au rythme du MJ.'),
    5: dict(bn=['you', 'La partie est lancée'], main=M('Ouvrir l’écran de jeu', 'Comme dans « Mener »', 'this.end()'),
            thought='Et c’est parti pour la porte nord.',
            why='La première scène part partout en même temps. À partir d’ici, l’écran de jeu du MJ prend le relais.'),
}
JS = component(5, ROWS,
               fresh='{ all: false, lines: 1 }',
               settled='{ all: m >= 2, lines: 3 }',
               go='if (m === 2) this.later(() => this.setState({ all: true }), 2500); if (m > 2) this.setState({ all: true }); if (m === 4) { this.later(() => this.setState({ lines: 2 }), 2500); this.later(() => this.setState({ lines: 3 }), 5000); } if (m > 4) this.setState({ lines: 3 });',
               vals='''seats: [{ p: 'borin', n: 'Marc', c: 'Borin · son activé', on: 'on' }, { p: 'lyra', n: 'Camille', c: 'Lyra · son activé', on: 'on' }, { p: 'sef', n: 'Hugo', c: m === 1 || (m === 2 && !s.all) ? 'en route…' : 'Sef · son activé', on: m === 1 || (m === 2 && !s.all) ? '' : 'on' }],
      presTxt: (m === 1 || (m === 2 && !s.all)) ? '2 sur 3' : '3 sur 3',
      tvTxt: m === 1 ? 'affiche K7QF, pas encore jumelée' : 'jumelée · suit la partie', tvOn: m === 1 ? '' : 'on',
      tvChk: m === 1 ? 'pend' : '', tvMark: m === 1 ? '·' : '✓', tvState: m === 1 ? ': pas encore' : ': Salon · TV',
      allChk: (m === 1 || (m === 2 && !s.all)) ? 'pend' : '', allMark: (m === 1 || (m === 2 && !s.all)) ? '·' : '✓', allTxt: (m === 1 || (m === 2 && !s.all)) ? 'Hugo n’est pas encore là' : 'Les trois joueurs ont le son',
      coTxt: { 1: 'Hugo a prévenu sur Discord : cinq minutes de retard. Il joue à distance, la TV ne change rien pour lui.', 2: 'La TV reprend la musique du salon. Rien d’autre tant que tu n’as pas lancé.', 3: 'Rien de secret ne part sur la TV : je vérifie chaque envoi.', 4: 'Je garde la scène de la porte nord prête. Lyra a son indice des oiseaux.', 5: 'Scène envoyée. Je propose la suite quand ils agiront.' }[m],
      tv_borin: '', tv_lyra: '', tv_sef: (m === 2 && !s.all) ? 'off' : '',
      l2: s.lines >= 2 ? 'on' : '', l3: s.lines >= 3 ? 'on' : '', ytTxt: 'muette sur les téléphones jusqu’au toucher',''',
               full=(2580, 1020), seul=(2468, 900))

END = iff('ended', '<div class="endc"><span class="lbl">Session 4 lancée</span><span class="ttl" style="font-size: 40px">La suite se joue sur « Mener ».</span><button type="button" class="go" style="width: 300px" onClick="{{restart}}"><span class="in"><span style="flex: 1"><span class="t">Rejouer le lancement</span></span><span class="chev">›</span></span></button></div>')
lap = lap[:-len('</div>')] + END + '</div>'
# The moment picker sits under the TV, in two columns, so the board stays one laptop wide plus the TV.
UNDER = '''<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px 20px; width: 988px">
<span class="lbl">Dans la tête de Romain, le MJ</span><span class="lbl">Pourquoi cet écran</span>
<div class="think">{{thought}}</div><div class="why" style="min-height: 92px">{{why}}</div>
<span class="lbl">Lancer la session · moment {{m}} sur 5</span><span></span>
<div class="prog"><sc-for list="{{dots}}" as="d" hint-placeholder-count="5"><button type="button" class="{{d.cls}}" onClick="{{d.pick}}">{{d.n}}</button></sc-for></div>
<div class="ctl"><button type="button" onClick="{{restart}}">Recommencer</button></div>
</div>'''
parts = lap + '<div style="display: flex; flex-direction: column; gap: 28px">' + tv + iff('full', UNDER, True) + '</div>'
board('Lancer-MJ', 'Lancer la session', 'ls-', parts, JS, 5, (2580, 1020),
      {"x": 4800, "y": 13600, "w": 2580, "h": 1020, "title": "Lancer · la TV et la session 4", "is_interactive": True}, css)
