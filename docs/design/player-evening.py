import json
def header(pv=31, mx=31, deg=0):
    d = f' degats="{deg}"' if deg else ''
    return f'''<div class="hd"><span class="pf"><dc-import name="Sprite" perso="borin" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><div style="display: flex; flex-direction: column; gap: 3px; flex: 1"><span class="ttl" style="font-size: 14px">Borin</span><dc-import name="Coeurs" pv="{pv}" max="{mx}" coeurs="8" px="2"{d} hint-size="151px,16px"></dc-import></div><span class="pv">{pv}<small> / {mx} PV</small></span></div>'''
def banner(kind, text, icon=''):
    return f'<div class="bn {kind}"><span class="bi">{icon}</span><span>{text}</span></div>'
def nav(on='Jeu'):
    return '<nav class="nav">' + ''.join(f'<button type="button" class="{"on" if n==on else ""}">{n}</button>' for n in ['Jeu', 'Personnage', 'Journal']) + '</nav>'
def phone(inner):
    return f'<div class="ph">{inner}</div>'
def btn(t, sub='', kind='clair', icon='none', w=342, stat='none', val=''):
    return f'<dc-import name="Bouton" titre="{t}" sous="{sub}" stat="{stat}" valeur="{val}" icon="{icon}" variante="{kind}" largeur="{w}" hint-size="{w}px,56px"></dc-import>'
def foot(inner): return f'<div class="ft">{inner}</div>'
def idle(t): return f'<div class="idle">{t}</div>'
EAR = '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M6 8.5a6 6 0 1 1 12 0c0 6-6 6-6 10a3 3 0 0 1-6 0M10 8.5a2 2 0 0 1 4 0"></path></svg>'
HAND = '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 11V6a2 2 0 0 0-4 0M14 10V4a2 2 0 0 0-4 0v2M10 10.5V6a2 2 0 0 0-4 0v8a8 8 0 0 0 16 0v-3a2 2 0 0 0-4 0"></path></svg>'
CLOCK = '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M12 7v5l3 2"></path></svg>'
STAR = '<svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 2l2.9 6.6 7.1.6-5.4 4.7 1.6 7L12 17.3 5.8 20.9l1.6-7L2 9.2l7.1-.6z"></path></svg>'
HEART = '<svg width="18" height="18" viewBox="0 0 24 24" fill="#FF4D5E" aria-hidden="true"><path d="M12 21s-8-5.2-8-11a5 5 0 0 1 8-4 5 5 0 0 1 8 4c0 5.8-8 11-8 11z"></path></svg>'
MUSIC = '<div class="mini"><span class="eq"><i></i><i></i><i></i></span><span style="flex: 1">Ombres de la crypte</span><span style="color: #8C8C8C">son activé</span></div>'

S = []
# 1 salon
S.append(dict(t='Il arrive', see='Le nom de la partie, l’heure, son personnage, qui est déjà là.', get='« Je suis bien au bon endroit, avec Borin. On attend le MJ. »', tap='Activer le son. C’est tout.', html=phone(header() + banner('wait', 'On attend le MJ', CLOCK) + '''
<div class="body" style="gap: 18px">
<div style="display: flex; flex-direction: column; gap: 4px"><span class="lbl">Ce soir · 20 h 30</span><span class="ttl" style="font-size: 26px; line-height: 1.1">La crypte des Valombre</span><span style="font-size: 13px; color: #A3A3A3">Session 3</span></div>
<div class="stage"><dc-import name="Perso" perso="borin" px="5" etat="aucun" hint-size="100px,130px"></dc-import><div style="display: flex; flex-direction: column; gap: 4px"><span class="lbl">Tu joues</span><span class="ttl" style="font-size: 22px">Borin</span><span style="font-size: 12px; color: #A3A3A3">nain guerrier · niveau 3</span><span class="link">Ce n’est pas toi ? Changer</span></div></div>
<div style="display: flex; flex-direction: column; gap: 8px"><span class="lbl">Autour de la table</span>
<div class="who"><span class="dot on"></span><b>Romain</b><span>le MJ</span></div>
<div class="who"><span class="dot on"></span><b>Camille</b><span>joue Lyra</span></div>
<div class="who"><span class="dot"></span><b>Hugo</b><span>joue Sef · pas encore là</span></div>
</div>
</div>''' + foot(btn('Activer le son', 'La musique de la partie passera ici', 'clair', 'plus')) + nav())))
# 2 precedemment
S.append(dict(t='On lui rappelle où on en est', see='Trois phrases sur la dernière fois et les deux indices déjà trouvés.', get='« Ah oui, la porte nord et le registre. »', tap='Rien. Il lit pendant que le MJ en parle.', html=phone(header() + banner('listen', 'Le MJ commence', EAR) + '''
<div class="body" style="gap: 16px">
<span class="ttl" style="font-size: 26px">Précédemment…</span>
<p class="nar" style="font-size: 20px">Vous êtes descendus dans la crypte des Valombre. Les crânes regardent tous la porte nord. Sef a juré avoir entendu chanter derrière.</p>
<div style="display: flex; flex-direction: column; gap: 8px"><span class="lbl">Ce que vous savez</span>
<div class="clue"><dc-import name="Objet" objet="parchemin" rarete="commune" taille="40" pips="false" hint-size="40px,40px"></dc-import><span><b>Le registre</b> : une page a été arrachée.</span></div>
<div class="clue"><dc-import name="Objet" objet="anneau" rarete="peu" taille="40" pips="false" hint-size="40px,40px"></dc-import><span><b>La bague du gardien</b> : gravée d’un corbeau.</span></div>
</div>
</div>''' + foot(idle('Rien à faire : écoute le MJ')) + nav())))
# 3 recit
S.append(dict(t='Le MJ raconte', see='Le lieu en grand, le texte qui s’écrit, la musique qui joue.', get='« J’écoute. Ce n’est pas encore à moi. »', tap='Rien. Il peut monter ou couper le son.', html=phone(header() + banner('listen', 'Le MJ raconte · écoute', EAR) + '''
<div class="body" style="gap: 14px">
<div class="art"><span class="ttl" style="font-size: 22px; color: #F2F2F2">La salle de l’autel</span><span class="tag">[ILLUSTRATION]</span></div>
<p class="nar typed" style="font-size: 20px">L’escalier s’arrête devant un autel de pierre noire. Des cendres encore tièdes. Quelqu’un est passé ici il y a moins d’une heure.</p>
''' + MUSIC + '''
</div>''' + foot(idle('Rien à faire pour l’instant')) + nav())))
# 4 que fais-tu
S.append(dict(t='C’est à lui de proposer', see='La question du MJ et trois idées en clair, plus « autre chose ».', get='« Je peux fouiller, lire, ou dire ce que je veux. »', tap='Une idée, puis « Proposer au MJ ».', html=phone(header() + banner('you', 'À vous : que faites-vous ?', HAND) + '''
<div class="body" style="gap: 12px">
<p class="nar" style="font-size: 19px">« Devant l’autel, que faites-vous ? »</p>
<div class="opt sel"><b>Fouiller l’autel</b><span>Chercher ce qui est caché. Tu es bon pour ça.</span></div>
<div class="opt"><b>Lire les inscriptions</b><span>Comprendre ce qui est écrit.</span></div>
<div class="opt"><b>Parler aux autres</b><span>Proposer un plan au groupe.</span></div>
<div class="opt free"><b>Autre chose…</b><span>Écris ou dis-le au MJ.</span></div>
<div class="others"><span class="lbl">Les autres</span><span>Lyra surveille l’escalier.</span></div>
</div>''' + foot(btn('Proposer au MJ', 'Fouiller l’autel', 'clair', 'fleche')) + nav())))
# 5 jet de de
S.append(dict(t='Le MJ lui demande un jet', see='Le dé, ce qu’il faut faire (13 ou plus) et son bonus.', get='« Je lance. 16 + 4 = 20 : réussi. »', tap='Le dé. Une seule fois.', html=phone(header() + banner('you', 'À toi : lance le dé', HAND) + '''
<div class="body" style="gap: 14px; align-items: center; text-align: center">
<span class="ttl" style="font-size: 22px">Fouiller l’autel</span>
<span style="font-size: 15px; color: #D4D4D4">Il te faut <b style="font-size: 20px; color: #F2F2F2">13 ou plus</b></span>
<dc-import name="De" de="d20" valeur="16" taille="150" hint-size="150px,150px"></dc-import>
<div class="res"><span>16</span><span class="op">+ 4</span><span class="op">=</span><span>20</span></div>
<span class="win">Réussi !</span>
<span style="font-size: 12px; color: #8C8C8C">+4, c’est ton bonus de Sagesse. Le MJ dit ce que tu trouves.</span>
</div>''' + foot(btn('Lancer le dé', 'Touche le dé ou ce bouton', 'clair', 'none')) + nav())))
# 6 indice
S.append(dict(t='Il trouve un indice', see='Une carte qui se retourne avec ce qu’il a trouvé.', get='« C’est important, c’est gardé pour moi et le groupe. »', tap='« Lire en grand » s’il veut. Sinon rien.', html=phone(header() + banner('star', 'Tu as trouvé quelque chose', STAR) + '''
<div class="body" style="gap: 14px; align-items: center">
<div class="flip"><dc-import name="GameCard" titre="La page arrachée" type="Indice" texte="Coincée sous l’autel : la page qui manquait au registre." valeur="" stat="none" icon="parchemin" w="220" hint-size="220px,308px"></dc-import></div>
<span style="font-size: 13px; color: #A3A3A3; text-align: center">Rangée dans le journal. Tout le groupe la voit.</span>
</div>''' + foot(btn('Lire en grand', '', 'sombre', 'oeil')) + nav())))
# 7 explorer
S.append(dict(t='Il se déplace', see='La carte en grand, son personnage, les cases où il peut aller en vert.', get='« Je peux aller jusqu’au vert. Le gris, on ne l’a pas encore vu. »', tap='Une case verte, puis « Y aller ».', html=phone(header() + banner('you', 'Exploration · déplace-toi', HAND) + '''
<div class="body" style="gap: 10px; padding-left: 0; padding-right: 0">
<div class="mapw"><div style="position: absolute; left: -12px; top: -8px"><dc-import name="Plan" theme="crypte" carte="salle" pitch="28" vue="joueur" hint-size="392px,280px"></dc-import></div>
<span class="reach" style="left: 72px; top: 188px"></span><span class="reach" style="left: 128px; top: 188px"></span><span class="reach" style="left: 156px; top: 188px"></span><span class="reach" style="left: 184px; top: 188px"></span><span class="reach" style="left: 72px; top: 216px"></span><span class="reach" style="left: 100px; top: 216px"></span><span class="reach" style="left: 128px; top: 216px"></span><span class="reach" style="left: 156px; top: 216px"></span><span class="reach" style="left: 184px; top: 216px"></span><span class="reach" style="left: 72px; top: 160px"></span><span class="reach dst" style="left: 212px; top: 188px"></span>
</div>
<div class="legend"><span><i class="g"></i>tu peux y aller</span><span><i class="f"></i>pas encore vu</span></div>
<div style="padding: 0 16px; font-size: 13px; color: #D4D4D4">Tu vas <b>4 cases</b> à droite, vers la porte.</div>
</div>''' + foot(btn('Y aller', '4 cases sur 6', 'clair', 'fleche')) + nav())))
# 8 attendre son tour
S.append(dict(t='Le combat commence, il attend', see='Qui joue maintenant, quand ce sera lui, ce qui vient de se passer.', get='« Le gobelin attaque Lyra. Moi, c’est après Sef. »', tap='Rien. Il peut préparer son action.', html=phone(header() + banner('wait', 'Combat · le gobelin joue', CLOCK) + '''
<div class="body" style="gap: 14px">
<div class="order">
<div class="o now"><span class="pf foe"><dc-import name="Sprite" perso="gobelin" px="2" anim="repos" hint-size="40px,52px"></dc-import></span><span>maintenant</span></div>
<div class="o"><span class="pf"><dc-import name="Sprite" perso="sef" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><span>Sef</span></div>
<div class="o me"><span class="pf"><dc-import name="Sprite" perso="borin" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><span>toi</span></div>
<div class="o"><span class="pf"><dc-import name="Sprite" perso="lyra" px="1" anim="repos" hint-size="20px,26px"></dc-import></span><span>Lyra</span></div>
</div>
<div class="feed"><span class="lbl">À l’instant</span><p>Le gobelin attaque <b>Lyra</b> à l’arc… <b>raté</b>.</p></div>
<div class="feed"><span class="lbl">Ton tour arrive</span><p>Dans <b>2 tours</b>. Tu pourras bouger de 6 cases puis attaquer.</p></div>
</div>''' + foot(btn('Préparer mon action', 'facultatif', 'sombre', 'oeil')) + nav())))
# 9 son tour
S.append(dict(t='C’est son tour', see='Trois étapes : bouger, agir, finir. La cible est déjà proposée.', get='« Je suis à côté du gobelin. J’attaque avec ma hache. »', tap='Une carte, puis « Attaquer le gobelin ».', html=phone(header(9, 31) + banner('turn', 'À toi, Borin !', HAND) + '''
<div class="body" style="gap: 10px">
<div class="steps"><span class="done">1 · Bouger ✓</span><span class="cur">2 · Agir</span><span>3 · Finir</span></div>
<div class="mapw small"><div style="position: absolute; left: -116px; top: -78px"><dc-import name="Plan" theme="crypte" carte="salle" pitch="34" vue="joueur" grille="false" combat="true" hint-size="476px,340px"></dc-import></div><span class="tgt" style="left: 51px; top: 157px"></span></div>
<span class="lbl">Cible : gobelin, juste à côté</span>
<div class="acts">
<div class="act sel"><b>Hache</b><span>Touche sur 13+ (+5)</span><span>1d12 + 3 dégâts</span></div>
<div class="act"><b>Second souffle</b><span>Soigne 1d10 + 3</span><span>1 fois par combat</span></div>
<div class="act"><b>Parade</b><span>+2 en défense</span><span>jusqu’à ton tour</span></div>
</div>
</div>''' + foot(btn('Attaquer le gobelin', 'Hache · 1d12 + 3', 'clair', 'none', stat='atq', val='+5') + '<span class="link" style="text-align: center">Finir sans attaquer</span>') + nav())))
# 10 touche
S.append(dict(t='Il est touché', see='Ses cœurs qui éclatent, combien il en reste, ce qu’il peut faire.', get='« J’ai mal : 9 PV. Au prochain tour, je me soigne. »', tap='Rien. Le conseil est là pour plus tard.', html=phone('<div class="shake">' + header(9, 31, 6) + '</div>' + banner('hurt', 'Le gobelin te touche : −6 PV', HEART) + '''
<div class="body" style="gap: 16px; align-items: center; text-align: center">
<div class="shake"><dc-import name="Perso" perso="borin" px="5" anim="touche" etat="aucun" hint-size="100px,130px"></dc-import></div>
<span class="big">−6</span>
<span style="font-size: 16px">Il te reste <b>9 PV sur 31</b></span>
<div class="tip"><b>Tu es en danger.</b> À ton tour, ta carte <b>Second souffle</b> te rend 1d10 + 3 PV.</div>
</div>''' + foot(idle('Rien à faire : c’est au tour de Lyra')) + nav())))
# 11 butin
S.append(dict(t='Le butin', see='L’objet en grand, sa rareté, ce qu’il change, en une phrase.', get='« Une meilleure hache, légendaire. Je la prends. »', tap='« L’équiper » ou « La garder dans le sac ».', html=phone(header(9, 31) + banner('star', 'Butin !', STAR) + '''
<div class="body" style="gap: 14px; align-items: center; text-align: center">
<div class="beam"><dc-import name="Objet" objet="hache" rarete="leg" taille="140" hint-size="140px,140px"></dc-import></div>
<span class="ttl" style="font-size: 24px; color: #F3CC63">Hache des Valombre</span>
<span class="lbl" style="color: #F3CC63">Légendaire</span>
<span style="font-size: 15px">Mieux que ta hache : <b>+2 pour toucher</b>.</span>
<span style="font-size: 12px; color: #8C8C8C">Lyra reçoit l’Écu de la garde.</span>
</div>''' + foot(btn('L’équiper', 'Remplace ta hache', 'clair', 'epee') + btn('La garder dans le sac', '', 'sombre', 'none')) + nav())))
# 12 fin
S.append(dict(t='Fin de soirée', see='Ce qui s’est passé, ce que Borin a gagné, la prochaine date.', get='« Bonne soirée. On se revoit jeudi. »', tap='« À jeudi ! »', html=phone(header(31, 31) + banner('listen', 'Fin de la session', STAR) + '''
<div class="body" style="gap: 16px">
<span class="ttl" style="font-size: 26px">Ce soir</span>
<ul class="rec"><li>Vous avez trouvé la page arrachée du registre.</li><li>Vous avez vaincu les gobelins de la crypte.</li><li>La porte nord est ouverte. Quelque chose chante derrière.</li></ul>
<div class="gain"><dc-import name="Objet" objet="hache" rarete="leg" taille="48" pips="false" hint-size="48px,48px"></dc-import><div style="display: flex; flex-direction: column; gap: 6px; flex: 1"><span style="font-size: 13px">Borin · niveau 3 → 4 bientôt</span><dc-import name="Cases" stat="ini" valeur="8" max="10" h="10" largeur="200" hint-size="200px,16px"></dc-import></div></div>
<div class="next"><span class="lbl">Prochaine session</span><span class="ttl" style="font-size: 20px">Jeudi 10 octobre · 20 h 30</span></div>
</div>''' + foot(btn('À jeudi !', 'Ajouter à mon agenda', 'clair', 'none')) + nav('Journal'))))

css = r'''
body{margin:0;background:#0A0A0A}
.pm{font-family:'Chakra Petch',system-ui,sans-serif;color:#F2F2F2;background:radial-gradient(120% 40% at 50% 0%,#1C1C1C 0,#0C0C0C 55%,#050505 100%);-webkit-font-smoothing:antialiased}
.ttl{font-family:'Cinzel',Georgia,serif;font-weight:800;letter-spacing:.02em}
.nar{margin:0;font-family:'Cormorant Garamond',Georgia,serif;font-style:italic;font-weight:500;line-height:1.3;color:#E5E5E5}
.lbl{font-size:10px;font-weight:600;letter-spacing:.2em;text-transform:uppercase;color:#8C8C8C}
.grid{display:grid;grid-template-columns:repeat(4,390px);gap:60px 56px}
.mo{display:flex;flex-direction:column;gap:16px}
.mh{display:flex;align-items:baseline;gap:10px}
.mh b{font-family:'Cinzel',serif;font-size:13px;width:28px;height:28px;border-radius:7px;display:inline-grid;place-items:center;background:#EDEDED;color:#0A0A0A;box-shadow:0 2px 0 #6E6E6E;flex:none}
.mh span{font-family:'Cinzel',serif;font-weight:800;font-size:20px}
.cap{display:grid;grid-template-columns:76px 1fr;gap:6px 10px;font-size:13px;line-height:1.45;color:#D4D4D4}
.cap dt{font-size:10px;font-weight:700;letter-spacing:.16em;text-transform:uppercase;color:#8C8C8C;padding-top:3px}
.cap dd{margin:0}
.ph{position:relative;width:390px;height:844px;border-radius:44px;overflow:hidden;border:10px solid #050505;box-shadow:0 0 0 2px #2C2C2C,0 30px 60px rgba(0,0,0,.7);background:radial-gradient(130% 70% at 50% 0%,#1A1A1A 0,#0C0C0C 60%,#050505 100%);box-sizing:border-box;display:flex;flex-direction:column}
.hd{display:flex;align-items:center;gap:10px;padding:44px 16px 10px}
.pf{display:inline-grid;place-items:end center;border-radius:8px;background:linear-gradient(180deg,#262626,#121212);border:2px solid #3A3A3A;box-shadow:0 3px 0 #000;padding:2px 3px 0;flex:none}
.pf.foe{border-color:#F2F2F2;background:#0A0A0A}
.pv{font-weight:700;font-size:18px;color:#FF4D5E}
.pv small{font-size:11px;color:#8C8C8C;font-weight:600}
.bn{margin:0 12px;display:flex;align-items:center;gap:10px;padding:10px 14px;border-radius:12px;font-weight:700;font-size:15px}
.bn .bi{display:grid;place-items:center;flex:none}
.bn.listen{background:#161616;border:1px solid #2C2C2C;color:#D4D4D4}
.bn.wait{background:#0A0A0A;border:1.5px dashed #5E5E5E;color:#D4D4D4}
.bn.you{background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A;animation:bnPulse 1.6s ease-in-out infinite}
.bn.turn{background:#FFD60A;color:#0A0A0A;box-shadow:0 3px 0 #8A6F00;font-family:'Cinzel',serif;font-size:18px;animation:bnPulse 1.2s ease-in-out infinite}
.bn.star{background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.bn.hurt{background:#0A0A0A;border:1.5px solid #FF4D5E;color:#F2F2F2}
@keyframes bnPulse{0%,100%{transform:none}50%{transform:scale(1.02)}}
.body{flex:1;display:flex;flex-direction:column;padding:16px;min-height:0;overflow:hidden}
.ft{display:flex;flex-direction:column;gap:8px;padding:10px 14px 8px}
.idle{text-align:center;padding:16px;border-radius:12px;border:1.5px dashed #3A3A3A;color:#8C8C8C;font-size:13px;font-weight:600}
.nav{display:flex;border-top:1px solid #1F1F1F;background:#060606;padding:2px 8px 18px}
.nav button{flex:1;font:inherit;font-size:12px;color:#8C8C8C;background:none;border:0;min-height:44px;position:relative}
.nav button.on{color:#F2F2F2}
.nav button.on::before{content:'';position:absolute;top:0;left:50%;margin-left:-12px;width:24px;height:2px;background:#F2F2F2}
.link{font-size:12px;color:#A3A3A3;text-decoration:underline;text-underline-offset:3px}
.stage{display:flex;align-items:flex-end;gap:16px;padding:14px;border-radius:14px;background:radial-gradient(60% 30% at 30% 92%,rgba(255,255,255,.08),transparent),#141414;border:1px solid #2C2C2C}
.who{display:flex;align-items:center;gap:10px;font-size:14px;padding:8px 10px;border-radius:10px;background:#121212}
.who span:last-child{margin-left:auto;font-size:12px;color:#8C8C8C}
.dot{width:9px;height:9px;border-radius:50%;background:#3A3A3A;flex:none}
.dot.on{background:#F2F2F2;box-shadow:0 0 8px rgba(255,255,255,.6)}
.clue{display:flex;align-items:center;gap:12px;font-size:13px;padding:8px;border-radius:10px;background:#141414;border:1px solid #2C2C2C}
.art{height:200px;border-radius:14px;display:flex;align-items:flex-end;padding:14px;box-sizing:border-box;position:relative;background:radial-gradient(70% 80% at 50% 100%,rgba(10,10,10,.9),transparent),repeating-linear-gradient(135deg,#3A3A3A 0 2px,#2A2A2A 2px 7px);border:1px solid #2C2C2C}
.tag{position:absolute;right:10px;top:10px;font-size:9px;font-weight:700;letter-spacing:.14em;background:#0A0A0A;padding:3px 6px;border-radius:4px}
.typed{animation:typeIn 4s steps(6) infinite}
@keyframes typeIn{0%{clip-path:inset(0 0 100% 0)}60%,100%{clip-path:inset(0 0 0 0)}}
.mini{display:flex;align-items:center;gap:10px;font-size:12px;padding:10px 12px;border-radius:10px;background:#141414;border:1px solid #2C2C2C}
.eq{display:flex;gap:2px;align-items:flex-end;height:14px}
.eq i{width:3px;background:#F2F2F2;animation:eq .8s ease-in-out infinite}
.eq i:nth-child(2){animation-delay:-.3s}.eq i:nth-child(3){animation-delay:-.6s}
@keyframes eq{0%,100%{height:4px}50%{height:14px}}
.opt{display:flex;flex-direction:column;gap:2px;padding:12px 14px;border-radius:12px;background:#141414;border:1.5px solid #2C2C2C}
.opt b{font-family:'Cinzel',serif;font-size:16px}
.opt span{font-size:12px;color:#A3A3A3}
.opt.sel{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A;outline:1.5px solid #0A0A0A;outline-offset:-5px;box-shadow:0 3px 0 #8A8A8A}
.opt.sel span{color:#5A5A5A}
.opt.free{border-style:dashed}
.others{display:flex;gap:8px;align-items:baseline;font-size:12px;color:#A3A3A3;margin-top:4px}
.res{display:flex;gap:10px;align-items:baseline;font-weight:700;font-size:34px;animation:resIn 3.6s ease-out infinite}
.res .op{font-size:22px;color:#8C8C8C}
@keyframes resIn{0%,50%{opacity:0;transform:translateY(8px)}60%,100%{opacity:1;transform:none}}
.win{font-family:'Cinzel',serif;font-weight:800;font-size:28px;color:#0A0A0A;background:#F2F2F2;padding:4px 18px;border-radius:8px;box-shadow:0 4px 0 #6E6E6E;animation:resIn 3.6s ease-out infinite}
.flip{animation:flipIn 1.4s cubic-bezier(.2,.9,.25,1.1) both;perspective:900px}
@keyframes flipIn{0%{transform:rotateY(180deg) scale(.6);opacity:0}100%{transform:none;opacity:1}}
.mapw{position:relative;height:300px;overflow:hidden;border-top:1px solid #2C2C2C;border-bottom:1px solid #2C2C2C}
.mapw.small{height:190px;border-radius:12px;border:1px solid #2C2C2C}
.reach{position:absolute;width:28px;height:28px;background:rgba(46,230,166,.28);box-shadow:inset 0 0 0 1.5px rgba(46,230,166,.85);z-index:950;animation:rc 1.6s ease-in-out infinite}
.reach.dst{background:rgba(46,230,166,.6);box-shadow:inset 0 0 0 2px #F2F2F2}
@keyframes rc{50%{opacity:.6}}
.legend{display:flex;gap:16px;padding:0 16px;font-size:12px;color:#A3A3A3}
.legend i{display:inline-block;width:12px;height:12px;margin-right:6px;vertical-align:-1px}
.legend i.g{background:rgba(46,230,166,.5);box-shadow:inset 0 0 0 1.5px #2EE6A6}
.legend i.f{background:#070707;box-shadow:inset 0 0 0 1px #3A3A3A}
.order{display:flex;align-items:flex-end;gap:10px}
.o{display:flex;flex-direction:column;align-items:center;gap:6px;font-size:11px;color:#8C8C8C}
.o.now span:last-child{color:#F2F2F2;font-weight:700}
.o.now .pf{border-color:#F2F2F2;box-shadow:0 3px 0 #000,0 0 16px rgba(255,255,255,.35)}
.o.me span:last-child{color:#FFD60A;font-weight:700}
.o.me .pf{border-color:#FFD60A}
.feed{display:flex;flex-direction:column;gap:6px;padding:12px 14px;border-radius:12px;background:#141414;border:1px solid #2C2C2C}
.feed p{margin:0;font-size:14px;line-height:1.45}
.steps{display:flex;gap:6px}
.steps span{flex:1;text-align:center;font-size:12px;font-weight:700;padding:8px 4px;border-radius:9px;background:#141414;color:#6E6E6E;border:1px solid #2C2C2C}
.steps .done{color:#A3A3A3}
.steps .cur{background:#EDEDED;color:#0A0A0A;border-color:#0A0A0A}
.tgt{position:absolute;width:40px;height:40px;border-radius:50%;border:2px dashed #FF9F1C;z-index:960;animation:spin 4s linear infinite}
@keyframes spin{to{transform:rotate(360deg)}}
.acts{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}
.act{display:flex;flex-direction:column;gap:3px;padding:10px;border-radius:10px;background:#141414;border:1.5px solid #2C2C2C;font-size:11px;color:#A3A3A3}
.act b{font-family:'Cinzel',serif;font-size:14px;color:#F2F2F2}
.act.sel{background:#EDEDED;border-color:#0A0A0A;color:#5A5A5A;box-shadow:0 3px 0 #8A8A8A}
.act.sel b{color:#0A0A0A}
.shake{animation:shk 2.4s linear infinite}
@keyframes shk{0%,8%,100%{transform:none}2%{transform:translate(-5px,2px)}4%{transform:translate(5px,-2px)}6%{transform:translate(-3px,0)}}
.big{font-weight:700;font-size:56px;line-height:1;color:#FF4D5E;-webkit-text-stroke:3px #000;paint-order:stroke;text-shadow:0 4px 0 #000}
.tip{font-size:13px;line-height:1.5;padding:12px 14px;border-radius:12px;background:#141414;border:1px solid #2C2C2C;text-align:left}
.beam{position:relative;padding:20px}
.beam::before{content:'';position:absolute;left:50%;top:-40px;bottom:0;width:90px;margin-left:-45px;background:linear-gradient(0deg,rgba(255,226,140,.6),transparent);filter:blur(6px);animation:rc 2s ease-in-out infinite}
.rec{margin:0;padding-left:18px;display:flex;flex-direction:column;gap:8px;font-size:14px;line-height:1.45}
.gain{display:flex;align-items:center;gap:12px;padding:12px;border-radius:12px;background:#141414;border:1px solid #2C2C2C}
.next{display:flex;flex-direction:column;gap:4px;padding:14px;border-radius:12px;background:#EDEDED;color:#0A0A0A;box-shadow:0 3px 0 #8A8A8A}
.next .lbl{color:#5A5A5A}
.rule{display:flex;gap:12px;align-items:flex-start;font-size:13px;color:#D4D4D4;line-height:1.45}
.rule b{flex:none;width:24px;height:24px;border-radius:6px;background:#EDEDED;color:#0A0A0A;display:grid;place-items:center;font-family:'Cinzel',serif;box-shadow:0 2px 0 #6E6E6E}
@media (prefers-reduced-motion: reduce){.pm *{animation:none!important}}
'''
cells = ''.join(f'''<div class="mo"><div class="mh"><b>{i+1}</b><span>{m["t"]}</span></div>{m["html"]}<dl class="cap"><dt>Il voit</dt><dd>{m["see"]}</dd><dt>Il comprend</dt><dd>{m["get"]}</dd><dt>Il touche</dt><dd>{m["tap"]}</dd></dl></div>''' for i, m in enumerate(S))
html = f'''<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>La soirée de Marc</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@600;700;800&amp;family=Chakra+Petch:wght@400;500;600;700&amp;family=Cormorant+Garamond:ital,wght@1,500;1,600&amp;display=swap">
<style>{css}</style>
</helmet>
<div class="pm" style="width: 1920px; height: 3640px; position: relative; overflow: hidden">
<div style="padding: 48px 64px 28px; display: flex; gap: 60px; align-items: flex-end">
<div style="display: flex; flex-direction: column; gap: 10px; max-width: 900px">
<div class="lbl">Joueur · téléphone</div>
<h1 class="ttl" style="margin: 0; font-size: 50px">La soirée de Marc</h1>
<p style="margin: 0; font-size: 16px; color: #A3A3A3; line-height: 1.5">Marc n’a jamais joué. Il est sur son téléphone, il parle avec les autres sur Discord. Il joue Borin. Voici sa soirée, moment par moment.</p>
</div>
<div style="display: flex; flex-direction: column; gap: 10px; max-width: 760px">
<div class="rule"><b>1</b><span><strong style="color: #F2F2F2">En haut, un bandeau dit ce qui se passe.</strong> Gris : on écoute. Pointillés : on attend. Ivoire : à toi. Jaune : ton tour de combat.</span></div>
<div class="rule"><b>2</b><span><strong style="color: #F2F2F2">Au milieu, un seul sujet.</strong> Pas de stats à décoder : seulement ses PV, en cœurs et en chiffres.</span></div>
<div class="rule"><b>3</b><span><strong style="color: #F2F2F2">En bas, une seule action principale</strong>, écrite avec un verbe. Sinon : « Rien à faire ». Trois onglets : Jeu, Personnage, Journal.</span></div>
</div>
</div>
<div class="grid" style="padding: 0 64px">{cells}</div>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":1920,"height":3640}}}}'>
class Component extends DCLogic {{
  renderVals() {{
    return {{}};
  }}
}}
</script>
</body>
</html>
'''
open('design/project/Soiree-Marc.dc.html', 'w').write(html)
d = json.load(open('design/project/canvas.json'))
d['boards']['Soiree-Marc.dc.html'] = {"x": 2040, "y": 4720, "w": 1920, "h": 3640, "title": "Joueur · la soirée de Marc"}
if 'Soiree-Marc.dc.html' not in d['order']: d['order'].append('Soiree-Marc.dc.html')
json.dump(d, open('design/project/canvas.json', 'w'), ensure_ascii=False, indent=2)
print(len(html))
