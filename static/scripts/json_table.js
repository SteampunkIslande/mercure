/**
 * Génère un tableau HTML à partir d'un objet JSON avec le format:
 * {
 *   "header": ["colA", "colB", "colC"],
 *   "data": [
 *     [{"content": "line1", "href": null, "class": "custom-class"}, {"content": "somelink", "href": "/link/to/whatever"}],
 *     [{"content": "line2", "href": null}, {"content": "some otherlink", "href": "/link/to/whatever", "class": "highlight"}]
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
      "Format de données invalide. Attendu: {header: [...], data: [...]}"
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
    .map((col) => `<th>${col}</th>`)
    .join("")}</tr>`;

  // Créer les lignes de données
  const dataRows = table
    .map((row) => {
      const cells = row
        .map((cell) => {
          const cssClass = cell.class ? ` class="${cell.class}"` : "";
          if (cell.href && cell.href !== null) {
            return `<td${cssClass}><a href="${cell.href}">${cell.content}</a></td>`;
          } else {
            return `<td${cssClass}>${cell.content}</td>`;
          }
        })
        .join("");
      return `<tr>${cells}</tr>`;
    })
    .join("");

  return `<table class="table">${headerRow}${dataRows}</table>`;
}
