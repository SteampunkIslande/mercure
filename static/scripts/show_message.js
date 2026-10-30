// Single function to display messages in the dedicated divs
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
  // Hide all messages
  Object.values(divId).forEach((id) => {
    const el = document.getElementById(id);
    if (el) el.style.display = "none";
  });
  // Display the correct message
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
