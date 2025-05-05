package main

import (
	"C"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"sync"

	twitterscraper "github.com/imperatrona/twitter-scraper"
)

var scraper *twitterscraper.Scraper
var scraperMutex sync.Mutex

//export Initialize
func Initialize(ffiCookies *C.char) *C.char {
	// Cookies can be obtained from the web browser. The only two required
	// cookies known today are `auth_token` and `ct0`. The rest are optional.

	cookies, err := ffiDeserialize[[]*http.Cookie](ffiCookies)
	if err != nil {
		return ffiError(err)
	}

	scraperMutex.Lock()
	defer scraperMutex.Unlock()

	if scraper != nil {
		return ffiError(fmt.Errorf("already initialized in (Initialize was called twice)"))
	}

	scraper = twitterscraper.New()

	proxy := os.Getenv("PROXY_SERVER")
	if proxy != "" {
		panicIfErr(scraper.SetProxy(proxy))
	}

	// It's possible that authenticated requests can be, in this case we can
	// quickly switch to the open account mode.
	//
	// One such case: https://github.com/imperatrona/twitter-scraper/issues/47
	if os.Getenv("X_OPEN_ACCOUNT") == "true" {
		scraper.LoginOpenAccount()
	} else {
		scraper.SetCookies(*cookies)

		// This is required for the scraper to know we are logged in
		if !scraper.IsLoggedIn() {
			return ffiError(fmt.Errorf("failed to initialize (cookies may be invalid)"))
		}
	}

	return ffiOk(nil)
}

//export GetTweet
func GetTweet(ffiTweetId *C.char) *C.char {
	scraperMutex.Lock()
	defer scraperMutex.Unlock()

	if scraper == nil {
		return ffiError(fmt.Errorf(
			"not logged in (Login was not called successfully before GetTweet)",
		))
	}

	tweetId, err := ffiDeserialize[string](ffiTweetId)
	if err != nil {
		return ffiError(err)
	}

	tweet, err := scraper.GetTweet(*tweetId)

	if err != nil {
		return ffiError(err)
	}

	if tweet == nil {
		return ffiError(fmt.Errorf("tweet not found"))
	}

	// Remove these to avoid circular references,
	// and also we don't need this info in snowpity-tg app
	return ffiOk(map[string]interface{}{
		"name":              tweet.Name,
		"username":          tweet.Username,
		"photos":            tweet.Photos,
		"videos":            tweet.Videos,
		"gifs":              tweet.GIFs,
		"sensitive_content": tweet.SensitiveContent,
	})
}

func ffiOk(obj interface{}) *C.char {
	return allocCJsonString(map[string]interface{}{
		"Ok": obj,
	})
}

func ffiError(err error) *C.char {
	return allocCJsonString(map[string]interface{}{
		"Err": err.Error(),
	})
}

func allocCJsonString(obj interface{}) *C.char {
	bytes, err := json.Marshal(obj)
	if err != nil {
		return C.CString(fmt.Sprintf("failed to serialize to JSON: %s", err.Error()))
	}
	return C.CString(string(bytes))
}

// Based on https://dev.to/goncalorodrigues/using-go-generics-for-cleaner-code-4em1
func ffiDeserialize[T any](jsonChars *C.char) (*T, error) {
	jsonString := C.GoString(jsonChars)
	out := new(T)
	if err := json.Unmarshal([]byte(jsonString), out); err != nil {
		return nil, fmt.Errorf("invalid input: %s\nInput:\n%s", err.Error(), jsonString)
	}
	return out, nil
}

func panicIfErr(err error) {
	if err != nil {
		panic(err)
	}
}

func main() {
	// Can be used to test the code in this file with `go run main.go`
}
