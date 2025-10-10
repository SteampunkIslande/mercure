/**
 * Génère un tableau HTML à partir d'un objet JSON avec le format:
 * {
 *   "header": [
 *     {"content": "colA", "class": "content-column"},
 *     {"content": "colB", "class": "badge-column"},
 *     {"content": "colC", "class": "numeric-column"}
 *   ],
 *   "table": [
 *     [
 *       {"content": "line1", "href": null, "class": "custom-class", "td_class": "content-column"},
 *       {"content": "somelink", "href": "/link/to/whatever", "td_class": "badge-column"}
 *     ]
 *   ]
 * }
 */
function generateJsonTable(data, elementId) {
  const tableDiv = document.getElementById(elementId);
  if (!tableDiv) {
    console.error(`Élément avec l'ID '${elementId}' non trouvé`);
    return;
  }

  if (!data || !data.header || !data.table) {
    console.error(
      "Format de données invalide. Attendu: {header: [...], table: [...]}"
    );
    return;
  }

  tableDiv.innerHTML = createTableFromJson(data);
}

/**
 * Crée le HTML d'un tableau à partir des données JSON
 */
function createTableFromJson(jsonData) {
  const { header, table } = jsonData;

  // Créer l'en-tête
  const headerRow = `<tr>${header
    .map((col) => {
      const headerClass = col.class ? ` class="${col.class}"` : "";
      return `<th${headerClass}>${col.content}</th>`;
    })
    .join("")}</tr>`;

  // Créer les lignes de données
  const dataRows = table
    .map((row) => {
      const cells = row
        .map((cell) => {
          const tdClass = cell.td_class ? ` class="${cell.td_class}"` : "";
          const spanClass = cell.class ? ` class="${cell.class}"` : "";

          if (cell.href && cell.href !== null) {
            return `<td${tdClass}><a href="${cell.href}">${cell.content}</a></td>`;
          } else {
            return `<td${tdClass}><span${spanClass}>${cell.content}</span></td>`;
          }
        })
        .join("");
      return `<tr>${cells}</tr>`;
    })
    .join("");

  return `<table class="table">${headerRow}${dataRows}</table>`;
}
