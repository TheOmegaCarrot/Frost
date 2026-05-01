(function(){
var defined_nav={top:[{t:"Introduction to Frost",h:"introduction.html"},{t:"Language Reference",h:"language.html"},{t:"Standard Library",h:"stdlib/index.html"}],core:[{t:"Collections",h:"stdlib/collections.html"},{t:"Debug",h:"stdlib/debug.html"},{t:"Foreign Values",h:"stdlib/foreign-values.html"},{t:"Functions",h:"stdlib/functions.html"},{t:"Mutable Cell",h:"stdlib/mutable-cell.html"},{t:"Operators",h:"stdlib/operators.html"},{t:"Output",h:"stdlib/output.html"},{t:"Streams",h:"stdlib/streams.html"},{t:"Strings",h:"stdlib/strings.html"},{t:"Types",h:"stdlib/types.html"}],std:[{t:"CLI",h:"stdlib/cli.html"},{t:"Datetime",h:"stdlib/datetime.html"},{t:"Encoding",h:"stdlib/encoding.html"},{t:"Filesystem",h:"stdlib/fs.html"},{t:"IO",h:"stdlib/io.html"},{t:"JSON",h:"stdlib/json.html"},{t:"Math",h:"stdlib/math.html"},{t:"OS",h:"stdlib/os.html"},{t:"Random",h:"stdlib/random.html"},{t:"Regex",h:"stdlib/regex.html"},{t:"String",h:"stdlib/string.html"}],ext:[{t:"Compression",h:"stdlib/compression.html"},{t:"HTTP",h:"stdlib/http.html"},{t:"Hash",h:"stdlib/hash.html"},{t:"MessagePack",h:"stdlib/msgpack.html"},{t:"SQLite",h:"stdlib/sqlite.html"},{t:"TOML",h:"stdlib/toml.html"},{t:"Unsafe",h:"stdlib/unsafe.html"}]};
var depth=document.documentElement.dataset.depth||"0";
var prefix=depth==="1"?"../":"";
var nav=document.createElement("nav");
nav.id="sidebar";
var content=document.createElement("div");
content.className="nav-content";
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
content.appendChild(h);
}
var ul=document.createElement("ul");
items.forEach(function(e){
var li=document.createElement("li");
li.appendChild(makeLink(e));
ul.appendChild(li);
});
content.appendChild(ul);
}
addSection("",defined_nav.top);
addSection("Core",defined_nav.core);
addSection("Standard Modules",defined_nav.std);
addSection("Extensions",defined_nav.ext);
nav.appendChild(content);
var gh=document.createElement("div");
gh.className="nav-github";
var gha=document.createElement("a");
gha.href="https://github.com/theomegacarrot/Frost";
gha.target="_blank";
gha.innerHTML='<svg height="16" width="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg> GitHub';
gh.appendChild(gha);
nav.appendChild(gh);
document.body.insertBefore(nav,document.body.firstChild);
var btn=document.createElement("button");
btn.id="menu-toggle";
btn.innerHTML='<svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="3" y1="4" x2="15" y2="4"/><line x1="3" y1="9" x2="15" y2="9"/><line x1="3" y1="14" x2="15" y2="14"/></svg>';
var overlay=document.createElement("div");
overlay.className="nav-overlay";
function toggleMenu(){
nav.classList.toggle("open");
overlay.classList.toggle("open");
}
btn.addEventListener("click",toggleMenu);
overlay.addEventListener("click",toggleMenu);
document.body.insertBefore(overlay,document.body.firstChild);
document.body.insertBefore(btn,document.body.firstChild);
})();