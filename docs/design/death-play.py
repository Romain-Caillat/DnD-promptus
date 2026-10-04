"""Generate a character's death: Borin falls to the goblin chief (Mourir-Borin.dc.html).

A variant evening. Eight moments, the GM laptop and Marc's phone side by side: down at 0 HP,
death saves, a failed rescue, the GM's choice of target, the last roll, last words, and what
Marc does next. The dice decide, the GM keeps the last word. Run from the repository root.
"""
import sys
sys.path.insert(0, 'docs/design')
from kit import imp, when, iff, head, note, chk, cb, top, laptop, phone, side, component, board

css = r'''
.saves{display:grid;grid-template-columns:110px repeat(3,44px);gap:10px;align-items:center}
.saves > b{font-size:13px}
.sv{width:44px;height:44px;border-radius:50%;border:2px dashed #3A3A3A;box-sizing:border-box;display:grid;place-items:center;font-weight:800}
.sv.ok{border:0;background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.sv.ko{border:0;background:#FF4D5E;color:#0A0A0A;box-shadow:0 3px 0 #8A2A33}
.saves.sm{grid-template-columns:80px repeat(3,34px);gap:8px}
.saves.sm > .sv{width:34px;height:34px}
.row{display:flex;align-items:center;gap:10px;height:56px;padding:0 10px;margin:0 10px 4px;border-radius:10px;background:#121212;border:1px solid transparent;font-size:13px}
.row.now{border-color:#FFD60A;background:#1E1E1E}
.row.down{border-color:#FF4D5E}
.row > .who{display:flex;flex-direction:column;gap:2px;flex:1}
.row > .who > small{font-size:11px;color:#8C8C8C}
.row > .hp{font-weight:700;color:#FF4D5E}
.pick{display:grid;grid-template-columns:1fr 1fr;gap:12px}
.ch{display:flex;flex-direction:column;gap:6px;padding:14px;border-radius:12px;background:#141414;border:1.5px solid #2C2C2C;font-size:13px;line-height:1.45;color:#A3A3A3}
.ch > b{font-family:'Cinzel',serif;font-size:17px;color:#F2F2F2}
.ch.on{background:#EDEDED;border-color:#0A0A0A;color:#5A5A5A;box-shadow:0 3px 0 #8A8A8A}
.ch.on > b{color:#0A0A0A}
.epi{display:flex;flex-direction:column;align-items:center;gap:12px;padding:20px;border-radius:16px;background:radial-gradient(60% 40% at 50% 30%,rgba(255,77,94,.12),transparent),#0D0D0D;border:1px solid #2C2C2C;text-align:center}
.gray{filter:grayscale(1) brightness(.7)}
.words{padding:14px 16px;border-radius:12px;background:#0D0D0D;border:1.5px solid #F2F2F2;font-family:'Cormorant Garamond',serif;font-style:italic;font-size:19px;line-height:1.35;color:#F2F2F2}
.fall{display:flex;align-items:center;gap:12px;padding:10px 12px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;font-size:13px;line-height:1.4;color:#D4D4D4}
'''

SAVES = '<div class="saves {cls}"><b>Réussites</b><span class="sv {{{{s1}}}}"></span><span class="sv {{{{s2}}}}"></span><span class="sv {{{{s3}}}}"></span><b>Échecs</b><span class="sv {{{{f1}}}}"></span><span class="sv {{{{f2}}}}"></span><span class="sv {{{{f3}}}}"></span></div>'
saves = lambda sm=False: SAVES.format(cls='sm' if sm else '')
BORIN = lambda px=4, gray=False: f'<span class="{"gray" if gray else ""}">' + imp('Perso', (20 * px, 26 * px), perso='borin', px=px, etat='aucun') + '</span>'

C = []
C.append(when(1, head('Borin est à terre', 'coup critique du chef gobelin')
  + '<div class="row2" style="align-items: center">' + BORIN(5) + '<div class="sec" style="flex: 1"><span class="ttl" style="font-size: 22px">0 PV</span><span class="mut">Le chef gobelin a fait 21 dégâts. Borin tombe, inconscient. Il n’est pas mort : à son tour, Marc lance les jets contre la mort.</span></div></div>' + saves()
  + '<div class="rl"><span class="lbl">Règle</span><span>Trois réussites : il se stabilise. Trois échecs : il meurt. Un 20 le relève avec 1 PV, un 1 compte deux échecs. Un soin le relève tout de suite.</span></div>'))
C.append(when(2, head('Premier jet contre la mort', 'Marc lance sur son téléphone') + '<div class="row2" style="align-items: center">' + imp('De', (110, 110), de='d20', valeur=7, taille=110, anim='fixe') + '<div class="sec"><span style="font-size: 30px; font-weight: 700">7</span><span class="mut">Moins de 10 : un échec.</span></div></div>' + saves()))
C.append(when(3, head('Lyra tente de le soigner', 'Sagesse (Médecine) DD 10') + '<div class="row2" style="align-items: center">' + imp('Perso', (80, 104), perso='lyra', px=4, etat='aucun') + imp('De', (90, 90), de='d20', valeur=4, taille=90, anim='fixe') + '<div class="sec"><span style="font-size: 26px; font-weight: 700">4 + 2 = 6</span><span class="mut">Raté. Lyra n’a pas de potion, le chef gobelin est toujours là.</span></div></div>' + saves()))
C.append(when(4, head('Le tour du chef gobelin', 'c’est toi qui le joues')
  + '<div class="pick"><div class="ch"><b>Achever Borin</b><span>Une attaque contre un personnage à terre compte deux échecs. Borin meurt.</span><small>Les règles le permettent.</small></div>'
  '<div class="ch {{mercy}}"><b>Arracher la hache</b><span>Le chef veut la hache des Valombre, pas le nain. Il frappe Lyra et recule vers l’eau.</span><small>Le co-MJ le propose : c’est ce qu’il veut depuis le début.</small></div></div>' + saves()))
C.append(when(5, head('Le dernier jet', 'Marc lance') + '<div class="row2" style="align-items: center">' + imp('De', (110, 110), de='d20', valeur=1, taille=110, anim='fixe') + '<div class="sec"><span style="font-size: 30px; font-weight: 700; color: #FF4D5E">1</span><span class="mut">Un 1 naturel : deux échecs. Borin meurt.</span></div></div>' + saves()
  + iff('notConfirmed', '<div class="secret"><span class="lbl">Ta table, ta règle</span><span>Les dés ont parlé. Tu peux confirmer, ou décider d’une autre issue : tu as le dernier mot, les joueurs ne voient rien avant ton choix.</span></div>', True)))
C.append(when(6, head('Les derniers mots', 'écrits par Marc') + '<div class="row2" style="align-items: center">' + BORIN(5, True) + '<div class="words" style="flex: 1">« Dis à Dorn que je suis descendu le chercher. »</div></div>'
  + '<div class="rl"><span class="lbl">Sur la TV</span><span>Le portrait de Borin, ses derniers mots, puis le silence : la musique s’arrête cinq secondes.</span></div>'))
C.append(when(7, head('Et après', 'pour Marc et pour la campagne')
  + '<div class="fall">' + BORIN(2, True) + '<span><b>Borin</b>, nain guerrier, niveau 3. Tombé dans la crypte des Valombre, session 3. Entré dans la chronique, rubrique « Tombés ».</span></div>'
  + '<div class="hook on"><b>Accroche · gardée secrète</b><span>Le culte des cendres relève les morts. Borin pourrait revenir, comme Dorn. La hache reste près de son corps.</span><small>→ front Le culte des cendres</small></div>'
  + '<div class="rl"><span class="lbl">Marc</span><span>{{marcTxt}}</span></div>'))

ROWS_L = [('lyra', 'Lyra', 'Camille', '20 / 24', ''), ('borin', 'Borin', 'Marc', '{{bHp}}', 'down'), ('chef', 'Chef gobelin', 'toi', '18 / 27', 'now'), ('sef', 'Sef', 'Hugo', '17 / 24', '')]
LEFT = ('<section class="panel" style="flex: 1"><div class="ph"><span class="lbl" style="color: #F2F2F2">Le combat</span><span class="lbl">tour 4</span></div>'
        + ''.join(f'<div class="row {c}"><span class="pf" style="width: 32px; height: 42px">{imp("Sprite", (20, 26), perso=p, px=1, anim="repos")}</span><div class="who"><b>{n}</b><small>{w}</small></div><span class="hp">{h}</span></div>' for p, n, w, h, c in ROWS_L) + '</section>')
RIGHT = '<section class="panel"><div class="ph"><span class="lbl" style="color: #F2F2F2">Le co-MJ</span></div><div class="co"><div class="note"><span class="lbl">{{coLbl}}</span>{{coTxt}}</div></div></section>'
TOP = top('Les Cendres de Valombre', ['Jeu', 'Carte', 'Journal'], 'Jeu', '<span>Session 3 · variante</span>')
lap = laptop(TOP, LEFT, ''.join(C), RIGHT, cols='280px 1fr 340px')

P = {
    1: ('hurt', 'Tu es à terre', '<div class="epi">' + BORIN(5) + '<span class="ttl" style="font-size: 24px">0 PV</span><span style="font-size: 13px; color: #D4D4D4; line-height: 1.5">Borin est inconscient. Pas mort. À ton tour, tu lanceras les jets contre la mort.</span></div>' + saves(True), ''),
    2: ('turn', 'À toi : jet contre la mort', '<div style="display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center"><span>Il te faut <b>10 ou plus</b></span>' + imp('De', (130, 130), de='d20', valeur=7, taille=130, anim='fixe') + '<span style="font-size: 26px; font-weight: 700">7 : un échec</span></div>' + saves(True), ''),
    3: ('wait', 'Lyra essaie de te soigner…', '<div class="epi">' + imp('Perso', (80, 104), perso='lyra', px=4, etat='aucun') + '<span style="font-size: 13px; color: #D4D4D4">Raté. Elle n’a pas de potion.</span></div>' + saves(True), ''),
    4: ('wait', 'Le chef gobelin joue…', '<div class="epi">' + imp('Perso', (80, 104), perso='chef', px=4, etat='aucun') + '<span style="font-size: 13px; color: #D4D4D4">{{chefPh}}</span></div>' + saves(True), ''),
    5: ('hurt', '{{p5}}', '<div style="display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center">' + imp('De', (130, 130), de='d20', valeur=1, taille=130, anim='fixe') + '<span style="font-size: 26px; font-weight: 700; color: #FF4D5E">1 : deux échecs</span>'
        + iff('confirmed', '<span class="ttl" style="font-size: 26px">Borin est tombé.</span>') + iff('notConfirmed', '<span class="mut">Romain regarde ce qui se passe…</span>', True) + '</div>' + saves(True), ''),
    6: ('you', 'Ses derniers mots', '<div class="epi" style="padding: 14px">' + BORIN(4, True) + '<span class="ttl" style="font-size: 20px">Borin</span></div><span class="lbl">Ce que Borin dit en tombant</span><div class="words">« Dis à Dorn que je suis descendu le chercher. »</div><span class="mut">Lus à la table et gardés dans la chronique.</span>',
        iff('notSaid', cb('Dire ses derniers mots', 'Sur la TV, pour toute la table', act='say'), True) + iff('said', '<div class="sent">Envoyé. Silence à la table.</div>')),
    7: ('listen', 'Et maintenant ?', '<span class="ttl" style="font-size: 22px">Tu restes à la table</span>'
        '<div class="ch {{c1}}"><b>Regarder ce soir</b><span>Tu suis la fin de la soirée comme spectateur, la TV et le journal.</span></div>'
        '<div class="ch {{c2}}"><b>Créer un nouveau personnage</b><span>Niveau 3, comme le groupe. Romain le fait entrer à la scène suivante.</span></div>'
        '<div class="ch"><b>Léguer ses objets</b><span>La hache reste où Borin est tombé. Le reste va à qui tu veux.</span></div>', ''),
}
ph = phone(P)

M = lambda lbl, sub, act, on='true': f'=[{lbl!r}, {sub!r}, {on}, () => {{ {act} }}]'
ROWS = {
    1: dict(bn=['hurt', 'Borin tombe à 0 PV'], main=M('Continuer le combat', 'Au tour de Borin : jet contre la mort', 'this.go(2)'),
            thought='21 dégâts. Je n’ai pas truqué le dé, je ne vais pas commencer.', coLbl='Le co-MJ', coTxt='Borin est à terre, pas mort. Personne n’a de potion ; Lyra peut tenter Médecine.',
            why='Tomber à 0 PV n’est pas mourir. L’écran dit clairement à tous ce qui se passe, et au joueur ce qu’il fera à son tour.'),
    2: dict(bn=['you', 'Un échec'], main=M('Au tour de Lyra', 'Elle peut tenter de le soigner', 'this.go(3)'),
            thought='Un échec. Marc ne dit plus rien.', coLbl='Le co-MJ', coTxt='1 échec, 0 réussite.',
            why='Le jet contre la mort se lance sur le téléphone du joueur, comme les autres. Le serveur tient le compte ; chacun le voit en cases.'),
    3: dict(bn=['you', 'Lyra rate son soin'], main=M('Au tour du chef gobelin', 'Que fait-il ?', 'this.go(4)'),
            thought='Camille a fait 4. La table hurle.', coLbl='Le co-MJ', coTxt='Le soin a échoué. Le chef joue maintenant.',
            why='Les autres joueurs peuvent agir : soigner, stabiliser, porter. Leurs essais comptent autant que les dés du joueur à terre.'),
    4: dict(bn=['warn', 'À toi de jouer le chef gobelin'], main=M('Arracher la hache', 'Le chef frappe Lyra et recule', 'this.setState({ mercy: true }); this.later(() => this.go(5), 900)', '!s.mercy'),
            thought='=s.mercy ? "Il veut la hache. Il ne s’acharne pas." : "Je pourrais l’achever. Ce n’est pas ce que veut ce gobelin."', coLbl='Proposition', coTxt='Le chef veut la hache, pas le nain : c’est son objectif depuis le début. Mais les règles te laissent l’achever.',
            why='Le MJ joue les monstres : les règles disent ce qui est possible, le MJ choisit ce qui a du sens. Le co-MJ rappelle ce que veut ce monstre.'),
    5: dict(bn=['hurt', '=s.confirmed ? "Borin meurt" : "Trois échecs"'], main=M('Confirmer : Borin meurt', 'Ou décider d’une autre issue', 'this.setState({ confirmed: true })', '!s.confirmed'),
            thought='=s.confirmed ? "Il est mort. Marc a les larmes aux yeux, et c’est la plus belle soirée de la campagne." : "Un 1. Je ne triche pas. Pas ce soir."', coLbl='Le co-MJ', coTxt='Trois échecs. Rien n’est envoyé aux joueurs avant ton choix.',
            why='Les dés décident, mais la mort d’un personnage ne part pas sans le MJ : il confirme, ou il décide d’une autre issue. Les joueurs ne voient rien avant.'),
    6: dict(bn=['you', 'Les derniers mots de Borin'], main=M('Reprendre la scène', 'Le chef s’enfuit avec la hache ?', 'this.go(7)'),
            thought='Dorn. Il pense à son frère jusqu’au bout.', coLbl='Le co-MJ', coTxt='Je garde ses derniers mots pour la chronique et pour plus tard.',
            why='Le joueur écrit ses derniers mots. Ils passent sur la TV, en silence, et restent dans la chronique : la mort est un moment, pas un écran de fin.'),
    7: dict(bn=['you', 'Marc reste à la table'], main=M('Valider le nouveau personnage', 'Quand Marc l’aura créé', 'this.end()'),
            thought='Le culte relève les morts… Borin reviendra peut-être. Pas comme Marc l’espère.', coLbl='Accroche', coTxt='Je te propose une accroche secrète : le culte relève les morts. Rien n’est dit aux joueurs.',
            why='Après la mort, le joueur ne sort pas : il regarde, ou crée un nouveau personnage que le MJ fait entrer. Le co-MJ propose de tisser cette mort dans l’histoire.'),
}
JS = component(7, ROWS, fresh='{ mercy: false, confirmed: false, said: false, choice: 2 }', settled='{ mercy: m >= 4, confirmed: m >= 5, said: m >= 6 }',
               go='if (m > 4) this.setState({ mercy: true }); if (m > 5) this.setState({ confirmed: true, said: true });',
               vals='''...(() => { const f = m >= 5 ? 3 : m >= 2 ? 1 : 0, ok = 0; const o = {}; for (let i = 1; i <= 3; i++) { o['s' + i] = i <= ok ? 'ok' : ''; o['f' + i] = i <= f ? 'ko' : ''; } return o; })(),
      bHp: m >= 5 && s.confirmed ? 'mort' : '0 / 31', mercy: s.mercy ? 'on' : '', confirmed: s.confirmed, notConfirmed: !s.confirmed, said: s.said, notSaid: !s.said,
      chefPh: s.mercy ? 'Il frappe Lyra et recule avec la hache des Valombre.' : 'Il te regarde, puis regarde ta hache…', p5: s.confirmed ? 'Borin est tombé' : 'Ton dernier jet',
      c1: '', c2: 'on', marcTxt: 'Il regarde la fin de la soirée, puis crée Brann, le cousin de Borin, à valider avant la session 4.',
      say: () => this.setState({ said: true }),''',
               full=(2450, 1020), seul=(1880, 900))
parts = lap + ph + iff('full', side('Romain, le MJ', 'La mort de Borin', 7), True)
board('Mourir-Borin', 'La mort d’un personnage', 'mo-', parts, JS, 7, (2450, 1020),
      {"x": 0, "y": 17600, "w": 2450, "h": 1020, "title": "Mourir · la dernière soirée de Borin (variante)", "is_interactive": True}, css)
