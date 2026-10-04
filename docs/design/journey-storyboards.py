"""Generate the storyboards of the nine journey boards drawn last, and lay those boards out on the canvas.

Each storyboard is its playable board frozen on every moment (seul=true), with three captions.
The layout pass places each board next to its storyboard, three pairs per row, under a row title.
Run from the repository root, after the board scripts.
"""
import json, sys
sys.path.insert(0, 'docs/design')
from kit import storyboard

PHONE, LAP, DUO, LAUNCH, TAB = (390, 844), (1440, 900), (1880, 900), (2468, 900), (1180, 820)

SB = [
    dict(name='Creation-Joueur', src='Creer-Joueur', prefix='sbc-', seul=PHONE, box_w=390, cols=4, cap_keys=('Marc', 'Romain', 'Pourquoi'),
         title='Créer un personnage, en images', lbl='Storyboard · le joueur', h1='Marc crée Borin',
         intro='Du lien d’invitation à la fiche envoyée, sur son téléphone, en moins de deux minutes. Le personnage se voit dès le premier choix.',
         rules=[('Voir avant de lire.', 'Chaque choix change le nain à l’écran ; les chiffres viennent après.'),
                ('Signaler, jamais bloquer.', 'Une limite dépassée part quand même : le MJ décide.'),
                ('L’histoire à deux.', 'Le co-MJ pose des questions, le joueur répond ; rien n’est inventé à sa place.')],
         frames=[('Le peuple', 'Choisit nain, en une touche.', 'Ne voit rien encore.', 'Le peuple vient d’abord : il décide du corps et des bonus.'),
                 ('Le corps', 'Peau, cheveux, barbe longue.', '—', 'Le personnage se construit en couches, comme un jouet.'),
                 ('Tenue et arme', 'Casque à cornes, hache.', '—', 'L’arme choisie devient la carte « Hache » de sa main.'),
                 ('Couleurs et nom', 'Trois « au hasard », puis Borin.', '—', 'Le hasard débloque ceux qui hésitent.'),
                 ('La classe', 'Guerrier.', '—', 'La classe donne les cartes de départ, déjà calculées.'),
                 ('Une limite', 'Met 18 en Force, voit l’orange.', 'Verra le signal en relisant.', 'Le joueur sait ce qu’il dépasse, le MJ aussi.'),
                 ('L’histoire', 'Répond au co-MJ : un frère disparu.', 'Recevra une accroche secrète.', 'Les réponses du joueur nourrissent la campagne.'),
                 ('Envoyé', 'Attend la validation, 1 min 48.', 'Reçoit la fiche à relire.', 'La création finit chez le MJ, qui a le dernier mot.')]),
    dict(name='Entre-Sessions', src='Entre-Joueur', prefix='sbe-', seul=PHONE, box_w=390, cols=4, cap_keys=('Marc', 'Romain', 'Pourquoi'),
         title='Entre deux sessions, en images', lbl='Storyboard · le joueur', h1='Marc entre deux soirées',
         intro='Du dernier coup de la session 3 au rappel de la session 4. Le jeu continue un peu sur le téléphone, sans rien exiger.',
         rules=[('Le niveau, tout de suite.', 'Monter de niveau se fait à chaud, avec la table.'),
                ('Rien à rattraper.', 'Récap et chronique se lisent en une minute.'),
                ('Un rendez-vous clair.', 'Les dates et le rappel ferment la boucle.')],
         frames=[('Fin de soirée', 'Voit l’expérience gagnée : niveau 4.', 'Clôt la session.', 'La récompense arrive avant qu’on se déconnecte.'),
                 ('Niveau 4', 'Lance le dé de vie : 7 + 3. Nouvelle carte.', '—', 'Le dé ou la moyenne : le joueur choisit son risque.'),
                 ('Le récap', 'Relit la soirée le lendemain.', 'L’a validé la veille.', 'Le co-MJ écrit, le MJ relit avant l’envoi.'),
                 ('La chronique', 'Fait le lien entre Dorn et la mine.', 'Y a glissé une piste.', 'Le récit de la campagne se garde, page après page.'),
                 ('La fiche', 'Relit Borin, 41 PV, et ses cartes.', '—', 'La fiche se consulte hors session, sans rien casser.'),
                 ('Jeudi', 'Donne ses dates, reçoit le rappel.', 'Choisit le jeudi.', 'Le salon ouvre à l’heure dite ; le joueur n’a qu’à toucher.')]),
    dict(name='Lancement-MJ', src='Lancer-MJ', prefix='sbl-', seul=LAUNCH, box_w=720, cols=3, cap_keys=('Romain', 'La table', 'Pourquoi'),
         title='Lancer la session 4, en images', lbl='Storyboard · le MJ', h1='Romain lance la session 4',
         intro='Marc et Camille sont dans le salon de Romain, Hugo joue depuis Lyon. La TV s’appaire en un code, puis la soirée commence.',
         rules=[('Un code, pas un compte.', 'La TV s’appaire comme un Chromecast.'),
                ('La TV n’a pas de secret.', 'Elle ne montre que ce que les joueurs ont le droit de voir.'),
                ('Reprendre le fil.', '« Précédemment » remet tout le monde dans l’histoire.')],
         frames=[('Le code', 'Tape K7QF ou partage une fenêtre.', 'La TV affiche son code.', 'Une table mixte : TV au salon, Discord pour Hugo.'),
                 ('Tout le monde', 'Voit Hugo arriver, micro vérifié.', 'Hugo rejoint depuis Lyon.', 'Le MJ voit qui est là avant de lancer.'),
                 ('Ce que voit la TV', 'Vérifie : pas de notes, pas de secrets.', '—', 'La TV est un joueur de plus, jamais le MJ.'),
                 ('Précédemment', 'Lit le résumé, ligne à ligne.', 'Le résumé s’affiche à l’écran.', 'Le MJ garde la voix, l’écran soutient.'),
                 ('La porte nord', 'Envoie la première scène.', 'Tous voient la scène.', 'La session commence sur une image, pas un menu.')]),
    dict(name='Voyage-MJ', src='Voyager-MJ', prefix='sbv-', seul=DUO, box_w=720, cols=3, cap_keys=('Romain', 'Marc', 'Pourquoi'),
         title='Voyager, en images', lbl='Storyboard · le MJ et la table', h1='De Valombre à Morneval',
         intro='Trois jours de route en vingt minutes de jeu. La carte en hexagones porte le voyage, les joueurs votent, le MJ raconte.',
         rules=[('Les joueurs choisissent.', 'Deux routes, un vote, le MJ tranche.'),
                ('Par portions.', 'Le voyage avance par demi-journées, pas case à case.'),
                ('L’événement est proposé.', 'Le co-MJ tire, le MJ garde ou change.')],
         frames=[('Deux routes', 'Propose le col et la forêt.', 'Voit les deux routes sur sa carte.', 'Le choix appartient au groupe.'),
                 ('Le vote', 'Lit 2 contre 1 pour la forêt.', 'Vote le col, perd.', 'Un vote rapide, sans débat sans fin.'),
                 ('Jour 1', 'Lit un paysage, avance le pion.', 'Suit le pion.', 'Le voyage se raconte par portions.'),
                 ('Le colporteur', 'Garde l’événement proposé.', 'Rencontre un colporteur.', 'L’IA propose, le MJ garde.'),
                 ('Survie', 'Demande un jet de groupe.', 'Rate, Lyra sauve le groupe.', 'Un jet de groupe : la moitié suffit.'),
                 ('La garde', 'Laisse Borin choisir.', 'Voit les loups venir.', 'La nuit a ses tours de garde.'),
                 ('L’abbaye', 'Entre dans Morneval.', 'Voit la nouvelle carte.', 'L’arrivée ouvre la carte du lieu.')]),
    dict(name='Systeme-Regles', src='Regles-MJ', prefix='sbr-', seul=LAP, box_w=720, cols=3, cap_keys=('Romain', 'Le co-MJ', 'Pourquoi'),
         title='Régler le système, en images', lbl='Storyboard · le MJ', h1='Les règles de Valombre',
         intro='Romain part du SRD de D&amp;D 5e et l’ajuste pour sa table : cases, difficultés, actions, une règle maison, puis un combat simulé.',
         rules=[('Une copie, jamais le préréglage.', 'Les changements restent dans la campagne.'),
                ('En français.', 'Le MJ écrit sa règle, le co-MJ la formalise, le MJ relit.'),
                ('Tester avant de jouer.', 'Une simulation montre l’effet des choix.')],
         frames=[('Le préréglage', 'Personnalise le SRD.', 'Liste ce qu’il contient.', 'On part d’un système complet.'),
                 ('Les statistiques', 'Compte le mouvement en cases.', 'Convertit : 6 cases.', 'Les gemmes suivent les statistiques.'),
                 ('Les jets', 'Passe « facile » à 12.', 'Chiffre l’effet : 65 %.', 'Quatre difficultés, données d’un clic en jeu.'),
                 ('Les actions', 'Désactive « Saisir ».', 'Adapte le chef gobelin.', 'Les actions sont les cartes des joueurs.'),
                 ('Une règle maison', 'Écrit : les morts-vivants ont peur du feu.', 'La formalise, avec trois cas.', 'Le MJ valide ce qu’il lit.'),
                 ('La création', 'Fixe 27 points, niveau 1.', 'Prépare les cartes de départ.', 'Ces limites pilotent le créateur des joueurs.'),
                 ('La simulation', 'Verrouille la version 2.', 'Simule 200 combats : 87 %.', 'Le MJ voit l’effet avant la table.')]),
    dict(name='Editeur-Carte', src='Carte-MJ', prefix='sbk-', seul=LAP, box_w=720, cols=3, cap_keys=('Romain', 'Le co-MJ', 'Pourquoi'),
         title='Dessiner une carte, en images', lbl='Storyboard · le MJ', h1='La salle de l’autel',
         intro='Le co-MJ propose la salle à partir du graphe ; Romain la retouche outil par outil et vérifie ce que verront les joueurs.',
         rules=[('Jamais de page blanche.', 'Chaque lieu arrive avec une carte proposée.'),
                ('Caché veut dire non envoyé.', 'Le serveur garde les secrets, pas les téléphones.'),
                ('Les échelles se relient.', 'Chaque sortie ouvre la bonne carte en jeu.')],
         frames=[('La proposition', 'Décide de retoucher.', 'Dessine la salle depuis le graphe.', 'Le MJ part d’une proposition.'),
                 ('Murs et eau', 'Peint l’eau au sud.', 'Rappelle le terrain difficile.', 'Un terrain porte sa règle.'),
                 ('Portes et décors', 'Pose une porte secrète.', 'Propose de l’ajouter au graphe.', 'Les décors viennent du pack.'),
                 ('Objets cachés', 'Cache la page sous l’autel.', 'Rappelle l’indice 3 sur 3.', 'Un secret a son jet.'),
                 ('Lumières', 'Allume deux braseros.', 'Note que Borin voit dans le noir.', 'La lumière décide de ce qu’on voit.'),
                 ('Vue des joueurs', 'Vérifie le brouillard.', 'Confirme : rien d’envoyé.', 'Le MJ voit comme ses joueurs.'),
                 ('Les sorties', 'Valide la carte.', 'Propose la salle est.', 'Les sorties ouvrent les lieux suivants.')]),
    dict(name='Ordi-Sef', src='Joueur-Ordi', prefix='sbo-', seul=LAP, box_w=720, cols=3, cap_keys=('Hugo', 'Romain', 'Pourquoi'),
         title='Jouer sur ordinateur, en images', lbl='Storyboard · le joueur', h1='Hugo joue Sef sur son ordi',
         intro='Le même jeu que sur téléphone, déplié sur un grand écran : la scène, la main, la carte et le combat côte à côte, au clavier.',
         rules=[('Les mêmes règles.', 'Rien de plus sur ordinateur, tout est visible d’un coup.'),
                ('Le clavier aide.', 'Raccourcis pour les cartes, espace pour le dé.'),
                ('Le MJ valide.', 'Une action proposée attend toujours le MJ.')],
         frames=[('La scène', 'Écoute, tout est à l’écran.', 'Raconte.', 'Le grand écran montre tout sans onglets.'),
                 ('La main', 'Propose de fouiller le tiroir.', 'Reçoit la proposition.', 'Les cartes ont leur raccourci.'),
                 ('Le dé', 'Lance : 18.', 'Révèle le tiroir.', 'Le dé se lance au clic ou à l’espace.'),
                 ('La carte', 'Se glisse vers l’eau.', 'Voit Sef bouger.', 'Les cases à portée s’allument.'),
                 ('Le combat', 'Attaque sournoise : 7.', 'Valide les dégâts.', 'Le combat garde le même rythme.'),
                 ('Le butin', 'Met la cape d’ombre.', 'L’a cachée dans le tiroir.', 'Le butin devient une carte.')]),
    dict(name='Mort-Borin', src='Mourir-Borin', prefix='sbm-', seul=DUO, box_w=720, cols=3, cap_keys=('Romain', 'Marc', 'Pourquoi'),
         title='La mort d’un personnage, en images', lbl='Storyboard · variante', h1='La dernière soirée de Borin',
         intro='Une variante de la session 3 où le dé ne pardonne pas. Le MJ garde la main sur chaque étape ; Marc garde sa place à la table.',
         rules=[('Le dé décide, le MJ confirme.', 'Rien n’est automatique pour une mort.'),
                ('Un moment à lui.', 'Le joueur dit ses derniers mots.'),
                ('Personne ne quitte la table.', 'Spectateur, nouveau personnage, ou retour possible.')],
         frames=[('À terre', 'Annonce 21 dégâts.', 'Voit Borin tomber à 0.', 'À 0 PV, les jets contre la mort commencent.'),
                 ('Premier jet', 'Lit un échec.', 'Lance : 7.', 'Trois échecs, trois réussites : tout le monde suit.'),
                 ('Le soin raté', 'Voit Camille faire 4.', 'Attend.', 'Les alliés peuvent tenter de sauver.'),
                 ('Le chef gobelin', 'Choisit : il prend la hache.', 'Voit la hache partir.', 'Le MJ choisit le geste du monstre.'),
                 ('Le dernier jet', 'Confirme la mort.', 'Fait 1.', 'Le MJ confirme, rien n’est automatique.'),
                 ('Derniers mots', 'Reprend la scène.', 'Parle de Dorn.', 'Le joueur a la parole une dernière fois.'),
                 ('La suite', 'Valide Brann, garde une piste.', 'Crée Brann ou regarde.', 'Marc reste à la table.')]),
    dict(name='Tablette-Soiree', src='Tablette-MJ', prefix='sbt-', seul=TAB, box_w=720, cols=3, cap_keys=('Romain', 'La table', 'Pourquoi'),
         title='Mener sur tablette, en images', lbl='Storyboard · le MJ', h1='La session 3 du canapé',
         intro='La même soirée que sur ordinateur, sur une tablette tenue à deux mains : un rail au lieu des onglets, de grosses touches, la voix pour le co-MJ.',
         rules=[('Rien de plus petit qu’un doigt.', 'Chaque cible fait au moins 60 pixels.'),
                ('Au même endroit.', 'Le rail ne bouge jamais.'),
                ('La voix pour le co-MJ.', 'Dicter va plus vite que taper.')],
         frames=[('La scène', 'Mène du canapé.', 'Écoute.', 'Le rail remplace les onglets.'),
                 ('Une demande', 'Répond Sagesse 13 d’un pouce.', 'Camille lance : 18.', 'La demande arrive en carte.'),
                 ('La carte au doigt', 'Lève le brouillard du doigt.', 'Voit la salle est.', 'Un doigt peint, deux déplacent.'),
                 ('Le combat', 'Valide l’attaque du gobelin.', 'Marc perd 6 PV.', 'Valider, changer, fuir : à portée de pouce.'),
                 ('Le co-MJ', 'Dicte : fais-le fuir.', 'Hugo voit le gobelin courir.', 'Le tiroir s’ouvre par-dessus la scène.')]),
]

SIZE = {}
for s in SB:
    SIZE[s['name']] = storyboard(s['name'], s['src'], s['title'], s['prefix'], s['lbl'], s['h1'], s['intro'], s['rules'], s['frames'], s['seul'], s['box_w'], s['cols'], s['cap_keys'],
               {"x": 0, "y": 0, "title": f"Storyboard · {s['h1']}", "is_interactive": False})

# Layout: rows of (board, storyboard) pairs, side by side, under a row title.
ROWS = [('Les écrans qui manquaient · créer, entre deux, lancer', ['Creer-Joueur', 'Entre-Joueur', 'Lancer-MJ']),
        ('Préparer en profondeur · voyager, régler, dessiner', ['Voyager-MJ', 'Regles-MJ', 'Carte-MJ']),
        ('Les variantes · ordinateur, mort, tablette', ['Joueur-Ordi', 'Mourir-Borin', 'Tablette-MJ'])]
SBOF = {s['src']: s['name'] for s in SB}
d = json.load(open('docs/design/canvas/canvas.json'))
b = d['boards']
y = 13600
for i, (label, srcs) in enumerate(ROWS):
    x, h = 0, 0
    for src in srcs:
        a, sb = b[f'{src}.dc.html'], b[f'{SBOF[src]}.dc.html']
        sb['w'], sb['h'] = SIZE[SBOF[src]]
        a.update(x=x, y=y)
        sb.update(x=x + a['w'] + 80, y=y)
        x = sb['x'] + sb['w'] + 240
        h = max(h, a['h'], sb['h'])
    d['notes'][f'row{5 + i}'] = {'kind': 'title1', 'maxW': x - 240, 'text': label, 'w': 240, 'x': 0, 'y': y - 300}
    y += h + 600
d['notes'].setdefault('row4', {'kind': 'title1', 'maxW': 4944, 'text': 'Inviter la table', 'w': 240, 'x': 0, 'y': 11100})
json.dump(d, open('docs/design/canvas/canvas.json', 'w'), ensure_ascii=False, indent=2)
print('laid out down to y', y)
