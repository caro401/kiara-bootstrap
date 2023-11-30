const {listen} = window.__TAURI__.event;



listen('logevent', (event) => {
    console.log(event)
    const list = document.querySelector("#steps")
    const newStep = document.createElement('li');
    newStep.appendChild(document.createTextNode(event.payload.message));
    list.appendChild(newStep);
})
