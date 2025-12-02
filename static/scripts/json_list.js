function createFormattedList(itemList, container, title) {
  if (
    !Array.isArray(itemList) ||
    itemList.some((item) => typeof item !== "string")
  ) {
    throw new Error("Input should be an array of strings");
  }

  // Ensure previous content is removed so repeated calls replace the list
  if (container) {
    container.innerHTML = "";
  }

  // Create a heading for the list
  const heading = document.createElement("h3");
  heading.textContent = title;
  heading.style.marginBottom = "10px";
  container.appendChild(heading);

  // Create search input
  const searchInput = document.createElement("input");
  searchInput.type = "text";
  searchInput.placeholder = "Rechercher un échantillon...";
  searchInput.style.width = "calc(100% - 8px)";
  searchInput.style.padding = "8px";
  searchInput.style.marginLeft = "4px";
  searchInput.style.marginRight = "4px";
  searchInput.style.marginBottom = "10px";
  searchInput.style.boxSizing = "border-box";
  searchInput.style.border = "1px solid #ccc";
  searchInput.style.borderRadius = "4px";
  container.appendChild(searchInput);

  // Create an unordered list element
  const ul = document.createElement("ul");

  // Add each item to the list with a nice presentation
  itemList.forEach((item) => {
    const li = document.createElement("li");
    li.textContent = item;
    li.style.padding = "5px 0";
    li.style.borderBottom = "1px solid #eee";
    ul.appendChild(li);
  });

  // Apply some basic styling to the unordered list
  ul.style.listStyleType = "none"; // Remove default bullets
  ul.style.paddingLeft = "0"; // Remove left padding

  // Append the styled list to the container
  container.appendChild(ul);

  // Add search functionality
  searchInput.addEventListener("keyup", function () {
    const filter = searchInput.value.toLowerCase();
    const listItems = ul.getElementsByTagName("li");

    for (let i = 0; i < listItems.length; i++) {
      const txtValue = listItems[i].textContent || listItems[i].innerText;
      if (txtValue.toLowerCase().indexOf(filter) > -1) {
        listItems[i].style.display = "";
      } else {
        listItems[i].style.display = "none";
      }
    }
  });
}
