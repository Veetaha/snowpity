package main

import (
	"C"
	"encoding/json"
	"fmt"
	"os"
	"sync"

	twitterscraper "github.com/imperatrona/twitter-scraper"
)

var scraper *twitterscraper.Scraper
var scraperMutex sync.Mutex

//export Initialize
func Initialize(ffiCookies *C.char) *C.char {
	// , err := ffiDeserialize[[]*http.Cookie](ffiCookies)
	// if err != nil {
	// 	return ffiError(err)
	// }

	scraperMutex.Lock()
	defer scraperMutex.Unlock()

	if scraper != nil {
		return ffiError(fmt.Errorf("already initialized in (Initialize was called twice)"))
	}

	scraper = twitterscraper.New()

	proxy, is_exist := os.LookupEnv("PROXY_SERVER")
	if is_exist {
		panicIfErr(scraper.SetProxy(proxy))
	}

	scraper.LoginOpenAccount()

	// scraper.SetCookies(*cookies)

	// // This is required for the scraper to know we are logged in
	// if !scraper.IsLoggedIn() {
	// 	return ffiError(fmt.Errorf("failed to initialize (cookies may be invalid)"))
	// }

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

// This is a small utility to get the cookies from a logged in session,
// which can be used to login in the Rust library.
func main() {
	scraper = twitterscraper.New()

	proxy, is_exist := os.LookupEnv("PROXY_SERVER")
	if is_exist {
		panicIfErr(scraper.SetProxy(proxy))
	}

	err := scraper.Login("username", "password", "mfa")
	panicIfErr(err)

	cookies := scraper.GetCookies()
	bytes, err := json.Marshal(cookies)
	panicIfErr(err)

	fmt.Println(string(bytes))
}

func panicIfErr(err error) {
	if err != nil {
		panic(err)
	}
}
