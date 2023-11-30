const {listen} = window.__TAURI__.event;



listen('logevent', (event) => {
    console.log(event)
    const list = document.querySelector("#steps")
    const newStep = document.createElement('li');
    newStep.appendChild(document.createTextNode(event.payload.message));
    list.appendChild(newStep);
})

listen('errorevent', (event) => {
    console.log(event)
    const html = document.querySelector("body")
    const err = document.createElement('pre');
    err.appendChild(document.createTextNode(event.payload.message));
    html.appendChild(err);
})
