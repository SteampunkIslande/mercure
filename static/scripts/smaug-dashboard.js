function onDropDown(element) {
        document.getElementById(element).classList.toggle("show");
    }
    function navigateTo(uri) {
        window.location.href = uri;
    }

    function closeDropdowns(event) {
        if (!event.target.matches('.dropbtn')) {
            var dropdowns = document.getElementsByClassName("dropdown-content");
            var i;
            for (i = 0; i < dropdowns.length; i++) {
                var openDropdown = dropdowns[i];
                if (openDropdown.classList.contains('show')) {
                    openDropdown.classList.remove('show');
                }
            }
        }
    }

    function openTab(evt, group_id) {
        var i, tabcontent, tablinks;

        tabcontent = document.getElementsByClassName("tabcontent");
        for (i = 0; i < tabcontent.length; i++) {
            tabcontent[i].style.display = "none";
        }

        tablinks = document.getElementsByClassName("tablinks");
        for (i = 0; i < tablinks.length; i++) {
            tablinks[i].className = tablinks[i].className.replace(" active", "");
        }

        document.getElementById(`forms-container-${group_id}`).style.display = "block";
        evt.currentTarget.className += " active";
    }

    const userGroups = window.smaugUserGroups;
    async function fetchAndRenderForms(groupId, listKey) {
        try {
            const response = await fetch(`/smaug/api/groups/forms/${groupId}`);
            const result = await response.json();

            if (!isApiSuccess(result)) {
                throw new Error(getApiMessage(result) || "Erreur inconnue");
            }

            const formsData = getApiData(result);
            const forms = formsData[listKey] || [];

            let table = document.getElementById(`forms-container-${groupId}`);
            if (!table) return;

            let container = table.querySelector('tbody') || table;

            forms.forEach(form => {
                const formBox = document.createElement('tr');
                let submitUrl = "#";

                // The switch/case handles different variants smoothly.
                switch (form.form_id) {
                    case "GitBranchAndPath": {
                        // FIX: Because of #[serde(flatten)], branch and file_path are on the root 'form' object, 
                        // not inside 'form.form_id' (which is just the string tag "GitBranchAndPath").
                        const { branch, file_path } = form;
                        submitUrl = `/smaug/runs/submit/branch/${branch}/${file_path}`;
                        break;
                    }
                    case "GitCommit": {
                        const { hash, file_path } = form;
                        submitUrl = `/smaug/runs/submit/commit/${hash}/${file_path}`;
                        break;
                    }
                    default:
                        console.warn(`Unknown form_id type: ${form.form_id}`);
                        break;
                }

                formBox.innerHTML = `
               <td class="content-column">${form.name}<br>${form.description}</td>
               <td class="numeric-column">
                   <a href="${submitUrl}" class="inline-table centered">Lancer un run</a>
               </td>
            `;
                container.appendChild(formBox);
            });
        } catch (error) {
            showMessage("error", `Erreur lors du chargement des formulaires du groupe ${groupId} : ${error.message}`);
        }
    }

    async function loadGroupForms() {
        const fetchTasks = [];

        if (userGroups && userGroups.length > 0) {
            // Queue all group fetches to run concurrently
            userGroups.forEach(group => {
                fetchTasks.push(fetchAndRenderForms(group.id, "with_group"));
            });
        } else {
            console.log("User has no groups!");
            const noGroupElement = document.getElementById("no-user-group");
            if (noGroupElement) noGroupElement.style.display = "block";
        }

        if (window.smaugIsAdmin) {
        // I moved this outside the (userGroups.length > 0) check.
        // Previously, if an admin had 0 groups, the "None" forms wouldn't load!
        fetchTasks.push(fetchAndRenderForms("None", "without_group"));
        }

        // Await all network requests simultaneously
        await Promise.all(fetchTasks);
    }
    async function lastRuns() {
        try {
            const response = await fetch(`/smaug/api/listruns?page=1&page_size=5`);
            const json = await response.json();
            if (isApiSuccess(json)) {
                const data = getApiData(json);
                if (data.table.length > 0) {
                    generateJsonTable(data, "latest-runs-table");
                } else {
                    document.getElementById("latest-runs-table").innerHTML = `<p>Aucune donnée à afficher.</p>`;
                }
            } else {
                showMessage("error", getApiMessage(json) || "Erreur inconnue");
            }
        } catch (error) {
            showMessage("error", error);
        }
    }

    function filterForms() {
        const queryString = document.querySelector('#search-forms input[name="q"]').value;
        const lowerQuery = queryString.toLowerCase();
        userGroups.forEach(group => {
            const table = document.getElementById(`forms-container-${group.id}`);
            const rows = table.getElementsByTagName('tr');
            for (let i = 1; i < rows.length; i++) {
                const formNameCell = rows[i].getElementsByTagName('td')[0];
                if (formNameCell) {
                    const formName = formNameCell.textContent || formNameCell.innerText;
                    if (formName.toLowerCase().indexOf(lowerQuery) > -1) {
                        rows[i].style.display = "";
                    } else {
                        rows[i].style.display = "none";
                    }
                }
            }
        });
    }

    async function init() {
        await loadGroupForms();
        await lastRuns();
    }

/* Ajouts purement visuels. Les fonctions historiques de chargement,
   lancement et filtrage ci-dessus restent inchangées. */
(function () {
  const hints = {
    demultiplexage: ['demultiplex', 'demux', 'bcl'],
    genetique: ['geneti', 'constitution', 'germinal', 'wes', 'wgs'],
    onco: ['onco', 'hemato', 'somati', 'cancer'],
    immunologie: ['immun'],
    autres: ['outil', 'autre', 'qualite', 'qc', 'annotation']
  };
  function norm(s) {
    return String(s || '').normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();
  }
  function searchInFormTables(terms) {
    // On ne consulte que les formulaires déjà chargés pour les groupes visibles.
    for (const group of userGroups) {
      const table = document.getElementById(`forms-container-${group.id}`);
      if (table && Array.from(table.querySelectorAll('tr')).slice(1)
          .some(row => terms.some(term => norm(row.textContent).includes(term)))) {
        return String(group.id);
      }
    }
    const noneTable = document.getElementById('forms-container-None');
    if (noneTable && Array.from(noneTable.querySelectorAll('tr')).slice(1)
        .some(row => terms.some(term => norm(row.textContent).includes(term)))) return 'None';
    return null;
  }
  function chooseCategory(category) {
    const terms = hints[category] || [];
    const feedback = document.getElementById('categoryFeedback');
    const id = searchInFormTables(terms);
    if (feedback) { feedback.hidden = Boolean(id); feedback.textContent = id ? '' : 'Aucun formulaire de cette catégorie n’est identifié dans vos groupes. Consultez la liste ci-dessous.'; }
    if (id !== null) {
      const button = document.getElementById(`tab-${id}`);
      if (button) openTab({currentTarget: button}, id);
      const table = document.getElementById(`forms-container-${id}`);
      const matching = terms.find(term => table && Array.from(table.querySelectorAll('tr')).slice(1).some(r => norm(r.textContent).includes(term)));
      const search = document.querySelector('#search-forms input[name="q"]');
      if (search) { search.value = matching || ''; filterForms(); }
    }
    document.getElementById('forms')?.scrollIntoView({behavior:'smooth',block:'start'});
  }
  document.addEventListener('DOMContentLoaded', () => {
    document.querySelectorAll('[data-category]').forEach(card => card.addEventListener('click', () => chooseCategory(card.dataset.category)));
    document.querySelectorAll('[data-target]').forEach(button => button.addEventListener('click', () => document.getElementById(button.dataset.target)?.scrollIntoView({behavior:'smooth'})));
    const global = document.getElementById('globalSearch');
    if (global) global.addEventListener('input', () => {
      const local = document.querySelector('#search-forms input[name="q"]');
      if (local) { local.value = global.value; filterForms(); }
    });
  });
})();
