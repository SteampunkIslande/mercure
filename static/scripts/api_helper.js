
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
