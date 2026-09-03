// Fonction unique pour afficher les messages dans les divs dédiées
function showMessage(type, message) {
  // type: 'error', 'info', 'success'
  const icons = {
    error: "error",
    info: "info",
    success: "check_circle",
  };
  const colors = {
    error: "red",
    info: "blue",
    success: "green",
  };
  const divId = {
    error: "error-message",
    info: "info-message",
    success: "success-message",
  };
  // Masquer tous les messages
  Object.values(divId).forEach((id) => {
    const el = document.getElementById(id);
    if (el) el.style.display = "none";
  });
  // Afficher le bon message
  const msgDiv = document.getElementById(divId[type]);
  if (msgDiv) {
    msgDiv.style.display = "block";
    msgDiv.innerHTML = `<span class="material-icons icon-align" style="color:${colors[type]};">${icons[type]}</span>${message}`;
    window.scrollTo(0, 0);
    if (type !== "error") {
      setTimeout(() => {
        msgDiv.style.display = "none";
      }, 2000);
    }
  }
}

function isApiSuccess(response) {
  if (!response || typeof response !== "object") {
    return false;
  }

  if (Object.prototype.hasOwnProperty.call(response, "success")) {
    return Boolean(response.success);
  }

  return Object.prototype.hasOwnProperty.call(response, "Success");
}

function getApiData(response) {
  if (!response || typeof response !== "object") {
    return undefined;
  }

  if (Object.prototype.hasOwnProperty.call(response, "data")) {
    return response.data;
  }

  if (response.Success && typeof response.Success === "object") {
    return response.Success.data;
  }

  return undefined;
}

function getApiMessage(response) {
  if (!response || typeof response !== "object") {
    return undefined;
  }

  if (Object.prototype.hasOwnProperty.call(response, "message")) {
    return response.message;
  }

  if (response.Failure && typeof response.Failure === "object") {
    return response.Failure.message;
  }

  return undefined;
}
