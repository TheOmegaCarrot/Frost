(function(){
var defined_nav={top:[{t:"Introduction to Frost",h:"introduction.html"},{t:"Language Reference",h:"language.html"},{t:"Standard Library",h:"stdlib/index.html"}],core:[{t:"Collections",h:"stdlib/collections.html"},{t:"Debug",h:"stdlib/debug.html"},{t:"Foreign Values",h:"stdlib/foreign-values.html"},{t:"Functions",h:"stdlib/functions.html"},{t:"Mutable Cell",h:"stdlib/mutable-cell.html"},{t:"Operators",h:"stdlib/operators.html"},{t:"Output",h:"stdlib/output.html"},{t:"Streams",h:"stdlib/streams.html"},{t:"Strings",h:"stdlib/strings.html"},{t:"Types",h:"stdlib/types.html"}],std:[{t:"CLI",h:"stdlib/cli.html"},{t:"Datetime",h:"stdlib/datetime.html"},{t:"Encoding",h:"stdlib/encoding.html"},{t:"Filesystem",h:"stdlib/fs.html"},{t:"IO",h:"stdlib/io.html"},{t:"JSON",h:"stdlib/json.html"},{t:"Math",h:"stdlib/math.html"},{t:"OS",h:"stdlib/os.html"},{t:"Random",h:"stdlib/random.html"},{t:"Regex",h:"stdlib/regex.html"},{t:"String",h:"stdlib/string.html"}],ext:[{t:"Compression",h:"stdlib/compression.html"},{t:"HTTP",h:"stdlib/http.html"},{t:"Hash",h:"stdlib/hash.html"},{t:"MessagePack",h:"stdlib/msgpack.html"},{t:"SQLite",h:"stdlib/sqlite.html"},{t:"TOML",h:"stdlib/toml.html"},{t:"Unsafe",h:"stdlib/unsafe.html"}]};
var depth=document.documentElement.dataset.depth||"0";
var prefix=depth==="1"?"../":"";
var nav=document.createElement("nav");
nav.id="sidebar";
function makeLink(e){
var a=document.createElement("a");
a.href=prefix+e.h;
a.textContent=e.t;
if(location.pathname.endsWith(e.h)||location.pathname.endsWith("/"+e.h))a.classList.add("active");
return a;
}
function addSection(label,items){
if(items.length===0)return;
if(label){
var h=document.createElement("div");
h.className="nav-heading";
h.textContent=label;
nav.appendChild(h);
}
var ul=document.createElement("ul");
items.forEach(function(e){
var li=document.createElement("li");
li.appendChild(makeLink(e));
ul.appendChild(li);
});
nav.appendChild(ul);
}
addSection("",defined_nav.top);
addSection("Core",defined_nav.core);
addSection("Standard Modules",defined_nav.std);
addSection("Extensions",defined_nav.ext);
document.body.insertBefore(nav,document.body.firstChild);
})();