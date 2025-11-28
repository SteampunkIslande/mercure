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
}
