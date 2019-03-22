const fibridge = require('fibridge-host');

const proxyAddress = window.location.hostname;
const portStr = window.location.port;

let port;
if (portStr === "") {
  port = 80;
}
else {
  port = parseInt(portStr, 10);
}

fibridge.createHoster({ proxyAddress, port, secure: false }).then((hoster) => {

  const allUrls = [];

  const uploadButton = document.getElementById('file_button');
  uploadButton.addEventListener('change', (e) => {
    const file = e.target.files[0];
    if (!file) {
      return;
    }

    const path = '/' + file.name;
    hoster.hostFile({ path, file });
    const fullPath = hoster.getHostedPath(path);
    const portStr = hoster.getPortStr();
    const url = `${window.location.protocol}//${proxyAddress}${portStr}${fullPath}`;

    if (allUrls.indexOf(url) === -1) {
      allUrls.push(url);
      const filesEl = document.getElementById('hosted_files');
      const div = document.createElement('div');
      div.style.fontSize = '22px';
      div.style.fontWeight = 'bold';
      div.innerHTML = `<a target='_blank' href=${url}>${url}</a>`;
      filesEl.appendChild(div);

      //const samtoolsDiv = document.createElement('div');
      //samtoolsDiv.style.fontSize = '22px';
      //samtoolsDiv.style.fontWeight = 'bold';
      //samtoolsDiv.innerHTML = `samtools view -H ${url}`;
      //filesEl.appendChild(samtoolsDiv);

      //const rangedDiv = document.createElement('div');
      //rangedDiv.style.fontSize = '22px';
      //rangedDiv.style.fontWeight = 'bold';
      //rangedDiv.innerHTML = `samtools view ${url} chr18`;
      //filesEl.appendChild(rangedDiv);

      //const curlDiv = document.createElement('div');
      //curlDiv.style.fontSize = '22px';
      //curlDiv.style.fontWeight = 'bold';
      ////curlDiv.innerHTML = `curl --limit-rate 10k ${url} > /dev/null`;
      ////curlDiv.innerHTML = `curl -H "Range: bytes=1-6" ${url} > file.bam`;
      //curlDiv.innerHTML = `curl ${url} > file.bam`;
      //filesEl.appendChild(curlDiv);
    }
  });
})
.catch((err) => {
  console.log(err);
});

