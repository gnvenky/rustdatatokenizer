curl -X POST -H "Content-Type: application/json" -d '{"input":"My age is My"}' http://127.0.0.1:8080/tokenize
{"tokenized":"Kdk5J68xSea7tynk NAb_XDias4-QqPMe tztkqqUFi39aQpV1 Kdk5J68xSea7tynk"}

curl -X POST -H "Content-Type: application/json" -d '{"input":"Kdk5J68xSea7tynk Kdk5J68xSea7tynk"}' http://127.0.0.1:8080/detokenize
{"detokenized":"My My"}
