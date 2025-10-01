function addNewGroup() {
  const newGroupName = document.getElementById("new_group_name").value.trim();
  if (!newGroupName) {
    alert("Veuillez entrer un nom pour le nouveau groupe.");
    return;
  }

  // Check for duplicate group names
  const existingGroups = Array.from(
    document.querySelectorAll('input[name="selected_groups"]')
  ).map((input) => input.nextElementSibling?.textContent?.trim());

  if (existingGroups.includes(newGroupName)) {
    alert("Ce nom de groupe existe déjà. Veuillez en choisir un autre.");
    return;
  }
  create_group(newGroupName);

  // Clear the input field
  document.getElementById("new_group_name").value = "";
}

function create_group(groupName) {
  // Create new group
  fetch(`/mercure/api/newgroup/${encodeURIComponent(groupName)}`)
    .then((response) => {
      if (!response.ok) {
        throw new Error(`Erreur lors de la création du groupe : ${groupName}`);
      }
      return response.json();
    })
    .then((data) => {
      if (data.success && data.data) {
        const { group_id, group_name } = data.data;
        if (group_id && group_name) {
          console.log(`Succesfully added ${group_name} with ID ${group_id}`);
        } else {
          throw new Error(`Réponse invalide pour le groupe : ${groupName}`);
        }
      } else {
        throw new Error(`Erreur dans la réponse : ${JSON.stringify(data)}`);
      }
    })
    .then(() => {
      load_groups();
    });
}

function load_groups() {
  const groupsContainer = document.getElementById("groups-container");
  groupsContainer.innerHTML = ""; // Clear existing groups

  fetch("/mercure/api/groups/list")
    .then((response) => {
      if (!response.ok) {
        throw new Error("Erreur lors du chargement des groupes.");
      }
      return response.json();
    })
    .then((data) => {
      if (data.success && Array.isArray(data.data)) {
        data.data.forEach((group) => {
          const groupDiv = document.createElement("div");
          groupDiv.className = "group-checkbox";
          groupDiv.innerHTML = `
            <input type="checkbox" id="group_${group.id}" name="selected_groups" value="${group.id}">
            <label for="group_${group.id}">${group.name}</label>
          `;
          groupsContainer.appendChild(groupDiv);
        });
      } else {
        throw new Error("Données de groupe invalides.");
      }
    })
    .catch((error) => {
      console.error(error);
      alert("Impossible de charger les groupes. Veuillez réessayer plus tard.");
    });
}
