# Routes disparues

## Liste des dossiers

Route originale: `/mercure/api/directories/list?<dirtype>`
Remplacée par: des routes spécialisées, par exemple, pour celle-là: `/mercure/api/aux/list_raw_dirs` ou `/mercure/api/aux/list_analysis_dir`

## Parsing des formulaires

Tout est repensé. Les routes suivantes sont supprimées pour ne pas être remplacées:

`/mercure/api/nextversion?<formname>`
`/mercure/parselauncher?<pipeline>&<launcher>`
`/mercure/disable/<formid>`
`/mercure/enable/<formid>`
`/mercure/newform`
`/mercure/api/formgroupedit`

Le modèle `FormDef` n'existe plus. Un autre sera créé, qui utilisera des définitions au format YAML.

Conséquence collatérale: suppression des templates:

`formgroupedit.html.j2`
