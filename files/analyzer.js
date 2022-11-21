const linkInput = document.getElementById('input')
const linkSubmit = document.getElementById('submit')


function escapeRegExp(stringToGoIntoTheRegex) {
    return stringToGoIntoTheRegex.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&');
}

const hrefRegex = new RegExp("^http(s)?:\/\/"+escapeRegExp(window.location.hostname.replace(/\/$/, "")) + "(:[\\d]+)?\\/(s\/)?[A-z\\d]{3}(\\.[A-z\\d]+)?$"); // if this regex matches, the URL is correct.
const shortRegex = new RegExp("^http(s)?:\/\/"+escapeRegExp(window.location.hostname.replace(/\/$/, "")) + "(:[\\d]+)?\\/s\/[A-z\\d]{3}(\\.[A-z\\d]+)?$"); // if this regex matches, the URL is correct.

let lock = false

linkInput.addEventListener('keyup', ev => {
  if (ev.key === 'Enter') {
    lookUpLink(linkInput.value)
  }
})

linkInput.addEventListener('paste', ev => {
  const paste = (ev.clipboardData || window.clipboardData).getData('text')
  lookUpLink(paste)
})

linkSubmit.addEventListener('click', ev => {
  lookUpLink(linkInput.value)
})

const lookUpLink = async (link) => {
  if (lock) return
  lock = true
  if (hrefRegex.test(link)) {
    console.log(`Getting info on ${link}`)
    linkInput.value = ''
    linkInput.placeholder = 'Looking up your link...'
    const anltcsLink = link.replace(/([A-z]*:\/\/([A-z\d\.:]?)*)/, "$1/a")
    const response = await fetch(anltcsLink)
    if (response.ok) {
      linkInput.classList.remove('error')
      const linkData = await response.text()
      linkInput.value = ''
      linkInput.placeholder = 'Link to look up...'
      const outRegex = /(Views: )?(\d*)[\n]?(Scrapes: )?(\d*)[\n]?(Created: )?(\d{2}\/\d{2}\/\d{2} \d*:\d*)/
      var views = linkData.replace(outRegex, "$2")
      var scrapes = linkData.replace(outRegex, "$4")
      var date = linkData.replace(outRegex, "$6")
      document.getElementById('results-table').classList.remove("hidden")
      document.getElementById('results').innerHTML += `<tr>
      <td>${link}</td>
      <td>${views}</td>
      <td>${scrapes}</td>
      <td>${scrapes}</td>
      <td>${date}</td>
      </tr>`
      linkInput.select()
    } else {
      linkInput.classList.add('error')
      linkInput.value = ''
      linkInput.placeholder = await response.text()
    }
  } else {
    linkInput.classList.add('error')
    linkInput.value = ''
    linkInput.placeholder = 'Are you absolutely sure that is a link?'
  }
  lock = false
}
