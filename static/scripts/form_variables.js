const VAR_ID_PREFIX = "formvar_";
const CUSTOM_OPTION = "__custom__";

function variableLabel(varDef) {
  return varDef.title || varDef.name;
}

function choiceFromItem(item) {
  if (typeof item === "string" || typeof item === "number") {
    return { value: String(item), label: String(item) };
  }
  if (item && typeof item === "object") {
    const rawValue =
      item.path !== undefined ? item.path :
      item.value !== undefined ? item.value :
      item.name !== undefined ? item.name :
      item.id;
    const label =
      item.name !== undefined ? item.name :
      item.title !== undefined ? item.title :
      item.path !== undefined ? item.path :
      String(rawValue);
    return { value: String(rawValue), label: String(label) };
  }
  return { value: String(item), label: String(item) };
}

async function fetchVariableChoices(source) {
  const response = await fetch(source, { headers: { Accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} en interrogeant ${source}`);
  }
  let payload = await response.json();
  if (
    payload &&
    typeof payload === "object" &&
    !Array.isArray(payload) &&
    Object.prototype.hasOwnProperty.call(payload, "success")
  ) {
    if (!isApiSuccess(payload)) {
      throw new Error(getApiMessage(payload) || `La source ${source} a répondu avec une erreur`);
    }
    payload = getApiData(payload);
  }
  if (!Array.isArray(payload)) {
    throw new Error(`Réponse inattendue de la source ${source}`);
  }
  return payload.map(choiceFromItem);
}

async function uploadVariableFile(file) {
  const formData = new FormData();
  formData.append("file", file);
  formData.append("file_name_base", file.name);
  const response = await fetch("/mercure/api/upload", { method: "POST", body: formData });
  const result = await response.json();
  if (!isApiSuccess(result)) {
    throw new Error(getApiMessage(result) || "Erreur lors de l'envoi du fichier");
  }
  return getApiData(result);
}

function placeholderOption() {
  const option = document.createElement("option");
  option.value = "";
  option.textContent = "--- Sélectionner une valeur ---";
  return option;
}

function appendTextWidget(div, varDef, currentValue) {
  const input = document.createElement("input");
  input.type = "text";
  input.id = VAR_ID_PREFIX + varDef.name;
  input.className = "var-input";
  input.value = currentValue || "";
  div.appendChild(input);
}

function appendFileWidget(div, varDef, currentValue) {
  const hidden = document.createElement("input");
  hidden.type = "hidden";
  hidden.id = VAR_ID_PREFIX + varDef.name;
  hidden.value = currentValue || "";

  const file = document.createElement("input");
  file.type = "file";
  file.dataset.role = "file";

  const status = document.createElement("span");
  status.className = "var-file-status";
  if (currentValue) {
    status.textContent = "Fichier actuel : " + currentValue;
  }

  file.addEventListener("change", async function () {
    if (!file.files.length) {
      return;
    }
    try {
      status.textContent = "Envoi du fichier en cours...";
      const path = await uploadVariableFile(file.files[0]);
      hidden.value = path;
      status.textContent = "Fichier envoyé : " + path;
    } catch (e) {
      hidden.value = "";
      status.textContent = "";
      showMessage("error", `Variable « ${variableLabel(varDef)} » : envoi du fichier impossible (${e.message})`);
    }
  });

  div.appendChild(file);
  div.appendChild(hidden);
  div.appendChild(status);
}

function fillSelectOptions(select, varDef, currentValue, choices) {
  select.appendChild(placeholderOption());
  const isKnown = choices.some((choice) => choice.value === currentValue);
  if (currentValue && !isKnown && !varDef.userdefined) {
    choices = [{ value: currentValue, label: `${currentValue} (valeur actuelle)` }, ...choices];
  }
  for (const choice of choices) {
    const option = document.createElement("option");
    option.value = choice.value;
    option.textContent = choice.label;
    if (currentValue === choice.value) {
      option.selected = true;
    }
    select.appendChild(option);
  }
  if (varDef.userdefined) {
    const custom = document.createElement("option");
    custom.value = CUSTOM_OPTION;
    custom.textContent = "Autre (valeur libre)...";
    select.appendChild(custom);
  }
}

async function appendSelectWidget(div, varDef, currentValue, choicesPromise) {
  const select = document.createElement("select");
  select.id = VAR_ID_PREFIX + varDef.name;
  select.className = "var-input";

  const status = document.createElement("span");
  status.className = "var-source-status";

  div.appendChild(select);
  div.appendChild(status);

  let choices;
  try {
    status.textContent = "Chargement des valeurs...";
    choices = await choicesPromise;
    status.textContent = "";
  } catch (e) {
    status.textContent = "Impossible de charger les valeurs : " + e.message;
    showMessage("error", `Variable « ${variableLabel(varDef)} » : ${e.message}`);
    const fallback = document.createElement("input");
    fallback.type = "text";
    fallback.className = "var-input";
    fallback.value = currentValue || "";
    fallback.placeholder = "Saisie manuelle (source indisponible)";
    select.remove();
    div.appendChild(fallback);
    return;
  }

  fillSelectOptions(select, varDef, currentValue, choices);

  if (varDef.userdefined) {
    const customInput = document.createElement("input");
    customInput.type = "text";
    customInput.dataset.role = "custom-value";
    customInput.className = "var-input";
    customInput.placeholder = "Valeur libre";
    customInput.style.display = "none";

    const isKnown = choices.some((choice) => choice.value === currentValue);
    if (currentValue && !isKnown) {
      select.value = CUSTOM_OPTION;
      customInput.value = currentValue;
      customInput.style.display = "inline-block";
    }

    select.addEventListener("change", function () {
      customInput.style.display = select.value === CUSTOM_OPTION ? "inline-block" : "none";
    });

    div.appendChild(customInput);
  }
}

async function renderFormVariables(schema, currentValues) {
  const container = document.getElementById("form-variables");
  if (!container) {
    return;
  }
  container.innerHTML = "";
  const values = currentValues || {};

  for (const varDef of schema.variables || []) {
    const div = document.createElement("div");
    div.className = "form-variable";
    div.dataset.varName = varDef.name;
    div.dataset.varType = varDef.type || "Text";

    const label = document.createElement("label");
    label.htmlFor = VAR_ID_PREFIX + varDef.name;
    label.textContent = variableLabel(varDef) + " :";
    div.appendChild(label);

    const currentValue = values[varDef.name] || "";

    if (varDef.type === "FromURL" && varDef.source) {
      await appendSelectWidget(div, varDef, currentValue, fetchVariableChoices(varDef.source));
    } else if (varDef.type === "ValuesList" && Array.isArray(varDef.values)) {
      await appendSelectWidget(div, varDef, currentValue, Promise.resolve(varDef.values.map(choiceFromItem)));
    } else if (varDef.type === "ExistingFile") {
      appendFileWidget(div, varDef, currentValue);
    } else {
      appendTextWidget(div, varDef, currentValue);
    }

    if (varDef.description) {
      const help = document.createElement("small");
      help.className = "var-description";
      help.textContent = varDef.description;
      div.appendChild(help);
    }

    container.appendChild(div);
  }
}

function variableValue(div) {
  const type = div.dataset.varType;
  if (type === "FromURL" || type === "ValuesList") {
    const select = div.querySelector("select");
    if (!select) {
      const fallback = div.querySelector("input[type='text']");
      return fallback ? fallback.value.trim() : "";
    }
    if (select.value === CUSTOM_OPTION) {
      const custom = div.querySelector("input[data-role='custom-value']");
      return custom ? custom.value.trim() : "";
    }
    return select.value;
  }
  if (type === "ExistingFile") {
    const hidden = div.querySelector("input[type='hidden']");
    return hidden ? hidden.value.trim() : "";
  }
  const input = div.querySelector("input[type='text']");
  return input ? input.value.trim() : "";
}

function collectFormVariables() {
  const collected = {};
  document.querySelectorAll("#form-variables .form-variable").forEach((div) => {
    collected[div.dataset.varName] = variableValue(div);
  });
  return collected;
}

function missingFormVariables() {
  const missing = [];
  document.querySelectorAll("#form-variables .form-variable").forEach((div) => {
    if (!variableValue(div)) {
      const label = div.querySelector("label");
      missing.push(label ? label.textContent.replace(/ :$/, "") : div.dataset.varName);
    }
  });
  return missing;
}
