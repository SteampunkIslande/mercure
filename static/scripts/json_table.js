// Génère une table HTML à partir d'un objet JSON selon le schéma donné
// parentName : nom de l'élément parent (clé principale dans l'objet)
// jsonObj : l'objet JSON à afficher
/**
 * Génère et insère une table HTML à partir d'un objet JSON selon le schéma donné.
 * @param {Object} jsonObj - L'objet JSON à afficher (clé = nom de colonne, valeur = tableau de cellules).
 * @param {string} parentName - id ou nom de l'élément HTML parent où insérer la table.
 */
function generateJsonTable(jsonObj, parentName) {
  const parent =
    document.getElementById(parentName) ||
    document.querySelector(`[name="${parentName}"]`);
  if (!parent) {
    throw new Error("Élément parent introuvable dans le DOM.");
  }
  const columns = Object.keys(jsonObj);
  if (columns.length === 0) return;

  const rowCount = jsonObj[columns[0]].length;
  const table = document.createElement("table");

  // En-tête
  const thead = document.createElement("thead");
  const headerRow = document.createElement("tr");
  columns.forEach((col) => {
    const th = document.createElement("th");
    th.textContent = col;
    headerRow.appendChild(th);
  });
  thead.appendChild(headerRow);
  table.appendChild(thead);

  // Corps
  const tbody = document.createElement("tbody");
  for (let i = 0; i < rowCount; i++) {
    const tr = document.createElement("tr");
    columns.forEach((col) => {
      const cellData = jsonObj[col][i];
      const td = document.createElement("td");
      if (cellData.type === "link" && cellData.href) {
        const a = document.createElement("a");
        a.href = cellData.href;
        a.textContent = cellData.text;
        td.appendChild(a);
      } else {
        td.textContent = cellData.text;
      }
      tr.appendChild(td);
    });
    tbody.appendChild(tr);
  }
  table.appendChild(tbody);

  parent.appendChild(table);
}

// Exemple d'utilisation :
// const json = {
//   "table": {
//     "Nom": [{type:"plain",text:"Alice"}, {type:"plain",text:"Bob"}],
//     "Profil": [{type:"link",href:"/alice",text:"Voir"}, {type:"link",href:"/bob",text:"Voir"}]
//   }
// };
// document.body.appendChild(generateJsonTable(json, "table"));
