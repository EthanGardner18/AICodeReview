// async_fetch_data.js
const fetch = require('node-fetch');

async function fetchData(url) {
    try {
        const response = await fetch(url);
        const data = await response.json();
        console.log(data);
    } catch (error) {
        console.error("Error fetching data: ", error);
    }
}

const url = 'https://api.example.com/data';
fetchData(url);

