const fs = require('fs');
const idl = require('../../target/idl/solpat.json');

fs.writeFileSync('./idl.json', JSON.stringify(idl));