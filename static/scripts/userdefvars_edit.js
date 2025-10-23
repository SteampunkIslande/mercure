function addUserVar() {
  const container = document.getElementById("user-vars-container");
  const div = document.createElement("div");
  div.className = "user-var-item";
  div.innerHTML = `
                <label for="var_name_${userVarCounter}">Nom de la variable :</label>
                <input type="text" id="var_name_${userVarCounter}" name="user_defined_vars[${userVarCounter}][name]" required>
                
                <div id="var_description_${userVarCounter}" style="display:none; font-style: italic; color: #666; margin: 5px 0;">
                    <strong>Description:</strong> <span id="var_description_text_${userVarCounter}"></span>
                </div>
                
                <label for="var_type_${userVarCounter}">Type :</label>
                <select id="var_type_${userVarCounter}" name="user_defined_vars[${userVarCounter}][type]" onchange="updateVarType(${userVarCounter})" required>
                    <option value="">-- Sélectionnez un type de variable --</option>
                    <option value="FromValuesList">Liste de valeurs autorisées</option>
                    <option value="Constant">Constante</option>
                    <option value="RunDefined">Défini à l'exécution (libre)</option>
                </select>
                
                <div id="var_values_${userVarCounter}" style="display:none;">
                    <label id="var_content_label_${userVarCounter}" for="var_content_${userVarCounter}">Contenu :</label>
                    <textarea id="var_content_${userVarCounter}" name="user_defined_vars[${userVarCounter}][content]" placeholder="" style="width: 100%; resize: none;" rows="5"></textarea>
                </div>
                
                <button type="button" onclick="removeUserVar(${userVarCounter})" style="display: inline">Supprimer</button>
            `;
  container.appendChild(div);
  userVarCounter++;
}

function removeUserVar(index) {
  const element = document.getElementById(`var_name_${index}`).parentElement;
  element.remove();
}

function updateVarType(index) {
  const typeSelect = document.getElementById(`var_type_${index}`);
  const valuesDiv = document.getElementById(`var_values_${index}`);
  const textarea = document.getElementById(`var_content_${index}`);
  const contentLabel = document.getElementById(`var_content_label_${index}`);
  const descriptionDiv = document.getElementById(`var_description_${index}`);

  if (typeSelect.value) {
    // Type sélectionné: cacher la description
    if (descriptionDiv) {
      descriptionDiv.style.display = "none";
    }

    valuesDiv.style.display = "block";
    contentLabel.style.display = "inline";

    if (typeSelect.value === "FromValuesList") {
      textarea.style.display = "inline";
      textarea.placeholder = "Entrez une valeur par ligne";
    } else if (typeSelect.value === "Constant") {
      textarea.style.display = "inline";
      textarea.placeholder = "Entrez la valeur constante";
    } else if (typeSelect.value === "RunDefined") {
      textarea.style.display = "none";
      contentLabel.style.display = "none";
    }
  } else {
    // Pas de type sélectionné: afficher la description si elle existe
    if (descriptionDiv) {
      const descText = document.getElementById(`var_description_text_${index}`);
      if (descText && descText.textContent) {
        descriptionDiv.style.display = "block";
      }
    }

    valuesDiv.style.display = "none";
    contentLabel.style.display = "none";
  }
}
