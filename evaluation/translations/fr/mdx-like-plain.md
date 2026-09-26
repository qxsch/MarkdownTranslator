# <a id="template-placeholders"></a>Espaces réservés dans les modèles

Les modèles utilisent des accolades pour les espaces réservés. Par exemple, {name} est remplacé par le nom de l’utilisateur et {date} par la date du jour.

Une comparaison telle que 1 < 2 ou a < b est du texte brut dans Markdown, tout comme une flèche telle que <- ou ->.

Écrivez {{double braces}} pour afficher une accolade littérale dans la sortie.

Les types génériques tels que List<string> et Map<K, V> apparaissent dans la référence du SDK.

L’objet de l’e-mail est « Bienvenue, {firstName} ! » par défaut. Vous pouvez le modifier comme vous le souhaitez, par exemple « Bonjour {firstName}, votre dossier {folder} est prêt ».

Un mot de type HTML tel que <placeholder> dans le texte courant n’est pas une balise dans la page rendue.
